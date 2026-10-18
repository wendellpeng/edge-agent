#![cfg_attr(windows, windows_subsystem = "windows")]

use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};

fn main() {
    let event_loop = EventLoopBuilder::new().build();

    // 创建托盘菜单
    let tray_menu = Menu::new();
    let open_item = MenuItem::new("打开控制台", true, None);
    let _ = tray_menu.append(&open_item);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let quit_item = MenuItem::new("退出", true, None);
    let _ = tray_menu.append(&quit_item);

    // 创建图标（32x32 绿色方块）
    let icon = create_icon();

    // 构建托盘图标
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("视觉巡检智能体\n点击打开控制台")
        .with_icon(icon)
        .build()
        .expect("创建托盘图标失败");

    event_loop.run(move |_event, _event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        // 处理托盘图标左键点击
        if let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. } = tray_event {
                open_browser();
            }
        }

        // 处理右键菜单事件
        if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            let id = menu_event.id;
            if id == open_item.id() {
                open_browser();
            } else if id == quit_item.id() {
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}

/// 打开浏览器访问控制台
fn open_browser() {
    let _ = std::process::Command::new("cmd")
        .args(&["/c", "start", "", "http://127.0.0.1:8080"])
        .spawn();
}

/// 创建一个简单的 32x32 绿色图标 (#7CB342)
fn create_icon() -> tray_icon::Icon {
    let width = 32u32;
    let height = 32u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            // 外圈深绿，内圈浅绿，做出简单层次感
            let is_border = x == 0 || x == width - 1 || y == 0 || y == height - 1;
            if is_border {
                rgba[idx] = 0x5A;     // R
                rgba[idx + 1] = 0x8E; // G
                rgba[idx + 2] = 0x2E; // B
            } else {
                rgba[idx] = 0x7C;     // R
                rgba[idx + 1] = 0xB3; // G
                rgba[idx + 2] = 0x42; // B
            }
            rgba[idx + 3] = 0xFF; // A
        }
    }

    tray_icon::Icon::from_rgba(rgba, width, height).expect("创建图标失败")
}
