use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

const READ_TIMEOUT_SECONDS: u64 = 10;
const HTTP_RETRY_SECONDS: u64 = 3;

#[derive(Clone, Debug)]
pub struct Reading {
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub status: String,
}

impl Reading {
    pub fn waiting() -> Self {
        Self {
            temperature: None,
            humidity: None,
            status: "正在查找数据源".to_string(),
        }
    }

    fn update(&mut self, temperature: f64, humidity: f64) {
        self.temperature = Some(temperature);
        self.humidity = Some(humidity);
        self.status = "已连接".to_string();
    }

    fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }
}

pub enum Source {
    Auto,
    Demo,
    Serial(String),
    Http(String),
}

pub fn run(source: Source, state: Arc<RwLock<Reading>>) {
    match source {
        Source::Demo => run_demo(state),
        Source::Serial(path) => loop {
            set_status(&state, format!("正在连接 {path}"));
            if let Err(error) = read_serial(&path, &state) {
                set_status(&state, format!("串口连接失败: {error}"));
            }
            thread::sleep(Duration::from_secs(2));
        },
        Source::Http(url) => loop {
            set_status(&state, "正在连接 HTTP 数据源");
            if let Err(error) = poll_http(&url, &state) {
                set_status(&state, format!("HTTP 连接失败: {error}"));
            }
            thread::sleep(Duration::from_secs(HTTP_RETRY_SECONDS));
        },
        Source::Auto => loop {
            let candidates = serial_candidates();
            if candidates.is_empty() {
                set_status(&state, "未发现 Pico 串口");
            }

            for path in candidates {
                set_status(&state, format!("正在连接 {}", path.display()));
                let _ = read_serial(&path.to_string_lossy(), &state);
            }

            thread::sleep(Duration::from_secs(2));
        },
    }
}

fn run_demo(state: Arc<RwLock<Reading>>) {
    let started = Instant::now();
    loop {
        let seconds = started.elapsed().as_secs_f64();
        let temperature = 24.6 + (seconds / 7.0).sin() * 1.2;
        let humidity = 56.0 + (seconds / 9.0).cos() * 5.0;

        if let Ok(mut reading) = state.write() {
            reading.update(temperature, humidity);
            reading.set_status("演示数据");
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn read_serial(path: &str, state: &Arc<RwLock<Reading>>) -> io::Result<()> {
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    configure_serial(&file)?;

    let mut reader = BufReader::new(file.try_clone()?);
    let mut line = String::new();
    let mut last_data = Instant::now();

    loop {
        line.clear();
        let count = reader.read_line(&mut line)?;
        if count == 0 {
            if last_data.elapsed() > Duration::from_secs(READ_TIMEOUT_SECONDS) {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "没有收到数据"));
            }
            continue;
        }

        if let Some((temperature, humidity)) = parse_reading(&line) {
            last_data = Instant::now();
            if let Ok(mut reading) = state.write() {
                reading.update(temperature, humidity);
            }
        }
    }
}

fn configure_serial(file: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;

    let fd = file.as_raw_fd();
    let mut settings: libc::termios = unsafe { std::mem::zeroed() };

    if unsafe { libc::tcgetattr(fd, &mut settings) } != 0 {
        return Err(io::Error::last_os_error());
    }

    unsafe {
        libc::cfmakeraw(&mut settings);
        libc::cfsetispeed(&mut settings, libc::B9600);
        libc::cfsetospeed(&mut settings, libc::B9600);
        settings.c_cc[libc::VMIN] = 0;
        settings.c_cc[libc::VTIME] = 10;

        if libc::tcsetattr(fd, libc::TCSANOW, &settings) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

fn poll_http(url: &str, state: &Arc<RwLock<Reading>>) -> Result<(), String> {
    let (host, port, path) = parse_http_url(url)?;
    let address = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .next()
        .ok_or_else(|| "无法解析主机名".to_string())?;

    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| error.to_string())?;

    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )
    .map_err(|error| error.to_string())?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| error.to_string())?;

    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or(&response);

    let (temperature, humidity) =
        parse_reading(body).ok_or_else(|| "JSON 中没有温湿度".to_string())?;

    if let Ok(mut reading) = state.write() {
        reading.update(temperature, humidity);
    }
    Ok(())
}

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "目前只支持 http:// 地址".to_string())?;
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, "/".to_string()),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse::<u16>()
                .map_err(|_| "HTTP 端口无效".to_string())?,
        ),
        None => (authority.to_string(), 80),
    };

    Ok((host, port, path))
}

fn serial_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(entries) = fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("cu.usbmodem") || name.to_uppercase().contains("JDY") {
                candidates.push(path);
            }
        }
    }

    candidates.sort_by_key(|path| {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.to_uppercase().contains("JDY") {
            0
        } else {
            1
        }
    });
    candidates
}

fn parse_reading(line: &str) -> Option<(f64, f64)> {
    if let (Some(temperature), Some(humidity)) = (
        number_after_key(line, "temperature"),
        number_after_key(line, "humidity"),
    ) {
        return Some((temperature, humidity));
    }

    let lower = line.to_ascii_lowercase();
    if let Some((_, values)) = lower.split_once("environment:") {
        let mut numbers = values
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter_map(|part| part.parse::<f64>().ok());
        if let (Some(temperature), Some(humidity)) = (numbers.next(), numbers.next()) {
            return Some((temperature, humidity));
        }
    }

    None
}

fn number_after_key(input: &str, key: &str) -> Option<f64> {
    let marker = format!("\"{key}\"");
    let start = input.find(&marker)? + marker.len();
    let value = input[start..].trim_start();
    let value = value.strip_prefix(':')?.trim_start();
    let end = value
        .find(|character: char| character == ',' || character == '}' || character.is_whitespace())
        .unwrap_or(value.len());
    value[..end].parse().ok()
}

fn set_status(state: &Arc<RwLock<Reading>>, status: impl Into<String>) {
    if let Ok(mut reading) = state.write() {
        reading.set_status(status);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_reading;

    #[test]
    fn parses_json() {
        assert_eq!(
            parse_reading(r#"{"temperature":24.6,"humidity":57}"#),
            Some((24.6, 57.0))
        );
    }

    #[test]
    fn parses_pico_debug_output() {
        assert_eq!(parse_reading("environment: 24 57"), Some((24.0, 57.0)));
    }
}
