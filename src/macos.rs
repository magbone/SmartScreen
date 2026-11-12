use std::ffi::{c_char, CString};
use std::sync::{Arc, RwLock};

use crate::sensor::Reading;

extern "C" {
    fn smart_screen_run();
    fn smart_screen_update(
        temperature: *const c_char,
        humidity: *const c_char,
        status: *const c_char,
        button_title: *const c_char,
    );
}

pub fn run() {
    unsafe {
        smart_screen_run();
    }
}

pub fn refresh(state: &Arc<RwLock<Reading>>) {
    let reading = match state.read() {
        Ok(reading) => reading.clone(),
        Err(_) => return,
    };

    let temperature = reading
        .temperature
        .map(|value| format!("温度    {value:.1} °C"))
        .unwrap_or_else(|| "温度    --".to_string());
    let humidity = reading
        .humidity
        .map(|value| format!("湿度    {value:.0} %"))
        .unwrap_or_else(|| "湿度    --".to_string());
    let status = format!("● {}", reading.status);
    let button = match (reading.temperature, reading.humidity) {
        (Some(temperature), Some(humidity)) => {
            format!("{temperature:.1}°C · {humidity:.0}%")
        }
        _ => "温湿度 --".to_string(),
    };

    let temperature = CString::new(temperature).unwrap();
    let humidity = CString::new(humidity).unwrap();
    let status = CString::new(status).unwrap();
    let button = CString::new(button).unwrap();

    unsafe {
        smart_screen_update(
            temperature.as_ptr(),
            humidity.as_ptr(),
            status.as_ptr(),
            button.as_ptr(),
        );
    }
}
