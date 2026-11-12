#import <AppKit/AppKit.h>

static NSStatusItem *gStatusItem;
static NSMenuItem *gTemperatureItem;
static NSMenuItem *gHumidityItem;
static NSMenuItem *gStateItem;

static NSMenuItem *AddItem(NSMenu *menu, NSString *title, BOOL enabled) {
    NSMenuItem *item = [[NSMenuItem alloc]
        initWithTitle:title
               action:nil
        keyEquivalent:@""];
    item.enabled = enabled;
    [menu addItem:item];
    return item;
}

void smart_screen_run(void) {
    @autoreleasepool {
        NSApplication *app = [NSApplication sharedApplication];
        [app setActivationPolicy:NSApplicationActivationPolicyAccessory];

        gStatusItem = [[NSStatusBar systemStatusBar]
            statusItemWithLength:NSVariableStatusItemLength];

        NSMenu *menu = [[NSMenu alloc] initWithTitle:@"Smart Screen"];
        menu.autoenablesItems = NO;

        NSMenuItem *header = AddItem(menu, @"Smart Screen", YES);
        header.enabled = NO;

        gTemperatureItem = AddItem(menu, @"温度    --", YES);
        gHumidityItem = AddItem(menu, @"湿度    --", YES);
        gStateItem = AddItem(menu, @"● 正在连接", YES);
        [menu addItem:[NSMenuItem separatorItem]];

        NSMenuItem *quit = [[NSMenuItem alloc]
            initWithTitle:@"退出 Smart Screen"
                   action:@selector(terminate:)
            keyEquivalent:@"q"];
        quit.target = app;
        [menu addItem:quit];

        gStatusItem.menu = menu;
        gStatusItem.button.title = @"温湿度 --";
        gStatusItem.button.toolTip = @"Smart Screen 温湿度";

        [app run];
    }
}

void smart_screen_update(
    const char *temperature,
    const char *humidity,
    const char *status,
    const char *button_title
) {
    if (gStatusItem == nil || gTemperatureItem == nil ||
        gHumidityItem == nil || gStateItem == nil) {
        return;
    }

    NSString *temperatureText =
        [NSString stringWithUTF8String:temperature ?: ""];
    NSString *humidityText =
        [NSString stringWithUTF8String:humidity ?: ""];
    NSString *statusText =
        [NSString stringWithUTF8String:status ?: ""];
    NSString *buttonText =
        [NSString stringWithUTF8String:button_title ?: ""];

    dispatch_async(dispatch_get_main_queue(), ^{
        gTemperatureItem.title = temperatureText;
        gHumidityItem.title = humidityText;
        gStateItem.title = statusText;
        gStatusItem.button.title = buttonText;
    });
}
