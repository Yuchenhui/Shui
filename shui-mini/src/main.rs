// Shui Mini — 极简喝水提醒。无 WebView，纯托盘 + 系统通知。
//
// 进程结构：
//   * 主线程跑 tao 事件循环（Windows 上同时是 Win32 消息泵）
//   * 一个独立 1 秒 tick 线程，向事件循环 send_event(Tick)
//   * tray-icon 的 MenuEvent 通过全局 channel 进来，转发到事件循环

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod lock;
mod stats;

use anyhow::Result;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use chrono::Local;
use std::time::Duration;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

const ICON_BYTES: &[u8] = include_bytes!("../assets/tray.png");

#[derive(Debug)]
enum UserEvent {
    Tick,
    Menu(MenuEvent),
}

#[allow(dead_code)]
struct Ids {
    // 仅占位用，"status" 和 "today_stat" 是 disabled item，不会触发点击事件。
    status: MenuId,
    today_stat: MenuId,
    drink: MenuId,
    pause: MenuId,
    remind_now: MenuId,
    reset: MenuId,
    interval_30: MenuId,
    interval_45: MenuId,
    interval_60: MenuId,
    interval_90: MenuId,
    workdays_only: MenuId,
    autostart: MenuId,
    open_config: MenuId,
    quit: MenuId,
}

struct App {
    config: config::Config,
    stats: stats::Stats,
    tray: TrayIcon,
    ids: Ids,
    paused: bool,
    locked: bool,
    elapsed_secs: u32,
    autostart: AutoLaunch,
}

impl App {
    fn new() -> Result<Self> {
        let config = config::Config::load();
        let stats = stats::Stats::load();

        let exe = std::env::current_exe()?;
        let autostart = AutoLaunchBuilder::new()
            .set_app_name("shui-mini")
            .set_app_path(&exe.to_string_lossy())
            .build()?;

        // 把 config 中的 autostart 与系统状态同步一次
        let _ = if config.autostart {
            autostart.enable()
        } else {
            autostart.disable()
        };

        let (menu, ids) = build_menu(&config, &stats, false, false);
        let tray = TrayIconBuilder::new()
            .with_tooltip("Shui Mini")
            .with_icon(load_icon()?)
            .with_menu(Box::new(menu))
            .build()?;

        Ok(Self {
            config,
            stats,
            tray,
            ids,
            paused: false,
            locked: false,
            elapsed_secs: 0,
            autostart,
        })
    }

    fn on_tick(&mut self) {
        if self.config.pause_on_lock {
            self.locked = lock::is_locked();
        } else {
            self.locked = false;
        }

        if self.paused {
            let _ = self.tray.set_tooltip(Some("Shui Mini — 已暂停"));
            return;
        }
        if self.locked {
            let _ = self.tray.set_tooltip(Some("Shui Mini — 已锁屏，计时暂停"));
            return;
        }

        let now = Local::now();
        if !self.config.in_work_window(now) {
            let _ = self.tray.set_tooltip(Some("Shui Mini — 非工作时段"));
            self.elapsed_secs = 0;
            return;
        }

        self.elapsed_secs += 1;
        let total = self.config.interval_minutes * 60;
        if self.elapsed_secs >= total {
            self.fire_reminder();
            self.elapsed_secs = 0;
        }
        let remaining = total.saturating_sub(self.elapsed_secs);
        let _ = self.tray.set_tooltip(Some(&format!(
            "下次提醒 {:02}:{:02} · 今日 {} 杯",
            remaining / 60,
            remaining % 60,
            self.stats.today()
        )));
    }

    fn fire_reminder(&self) {
        let _ = notify_rust::Notification::new()
            .summary("该喝水啦 💧")
            .body("起来活动一下，喝杯水吧")
            .timeout(notify_rust::Timeout::Milliseconds(8000))
            .appname("Shui Mini")
            .show();
    }

    fn on_menu(&mut self, ev: MenuEvent) -> Result<()> {
        let id = ev.id();
        if id == &self.ids.drink {
            let n = self.stats.add_today();
            let _ = self.stats.save();
            let _ = notify_rust::Notification::new()
                .summary("👍 已记录")
                .body(&format!("今日已喝 {} 杯", n))
                .timeout(notify_rust::Timeout::Milliseconds(2000))
                .appname("Shui Mini")
                .show();
            self.rebuild_menu();
        } else if id == &self.ids.pause {
            self.paused = !self.paused;
            self.rebuild_menu();
        } else if id == &self.ids.remind_now {
            self.fire_reminder();
            self.elapsed_secs = 0;
        } else if id == &self.ids.reset {
            self.elapsed_secs = 0;
        } else if id == &self.ids.open_config {
            if let Ok(p) = config::path() {
                let _ = std::fs::create_dir_all(p.parent().unwrap_or(&p));
                if !p.exists() {
                    let _ = self.config.save();
                }
                open_path(&p);
            }
        } else if id == &self.ids.quit {
            std::process::exit(0);
        } else if id == &self.ids.interval_30 {
            self.set_interval(30)?;
        } else if id == &self.ids.interval_45 {
            self.set_interval(45)?;
        } else if id == &self.ids.interval_60 {
            self.set_interval(60)?;
        } else if id == &self.ids.interval_90 {
            self.set_interval(90)?;
        } else if id == &self.ids.workdays_only {
            self.config.workdays_only = !self.config.workdays_only;
            let _ = self.config.save();
            self.rebuild_menu();
        } else if id == &self.ids.autostart {
            self.config.autostart = !self.config.autostart;
            if self.config.autostart {
                let _ = self.autostart.enable();
            } else {
                let _ = self.autostart.disable();
            }
            let _ = self.config.save();
            self.rebuild_menu();
        }
        Ok(())
    }

    fn set_interval(&mut self, m: u32) -> Result<()> {
        self.config.interval_minutes = m;
        self.elapsed_secs = 0;
        let _ = self.config.save();
        self.rebuild_menu();
        Ok(())
    }

    fn rebuild_menu(&mut self) {
        let (menu, ids) = build_menu(&self.config, &self.stats, self.paused, self.locked);
        self.tray.set_menu(Some(Box::new(menu)));
        self.ids = ids;
    }
}

fn build_menu(
    cfg: &config::Config,
    stats: &stats::Stats,
    paused: bool,
    locked: bool,
) -> (Menu, Ids) {
    let menu = Menu::new();

    let status_text = if paused {
        "● 已暂停".to_string()
    } else if locked {
        "● 已锁屏".to_string()
    } else {
        format!("● 间隔 {} 分钟", cfg.interval_minutes)
    };
    let status = MenuItem::new(status_text, false, None);
    let today_stat = MenuItem::new(format!("今日已喝 {} 杯", stats.today()), false, None);
    let _ = menu.append(&status);
    let _ = menu.append(&today_stat);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let drink = MenuItem::new("我喝了一杯 +1", true, None);
    let pause = MenuItem::new(if paused { "继续提醒" } else { "暂停提醒" }, true, None);
    let remind_now = MenuItem::new("立即提醒一次", true, None);
    let reset = MenuItem::new("重置倒计时", true, None);
    let _ = menu.append(&drink);
    let _ = menu.append(&pause);
    let _ = menu.append(&remind_now);
    let _ = menu.append(&reset);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let i30 = CheckMenuItem::new("30 分钟", true, cfg.interval_minutes == 30, None);
    let i45 = CheckMenuItem::new("45 分钟", true, cfg.interval_minutes == 45, None);
    let i60 = CheckMenuItem::new("60 分钟", true, cfg.interval_minutes == 60, None);
    let i90 = CheckMenuItem::new("90 分钟", true, cfg.interval_minutes == 90, None);
    let interval_sub = Submenu::new("提醒间隔", true);
    let _ = interval_sub.append(&i30);
    let _ = interval_sub.append(&i45);
    let _ = interval_sub.append(&i60);
    let _ = interval_sub.append(&i90);
    let _ = menu.append(&interval_sub);

    let workdays_only = CheckMenuItem::new("仅工作日提醒", true, cfg.workdays_only, None);
    let autostart = CheckMenuItem::new("开机自启", true, cfg.autostart, None);
    let _ = menu.append(&workdays_only);
    let _ = menu.append(&autostart);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let open_config = MenuItem::new("打开配置文件…", true, None);
    let quit = MenuItem::new("退出", true, None);
    let _ = menu.append(&open_config);
    let _ = menu.append(&quit);

    let ids = Ids {
        status: status.id().clone(),
        today_stat: today_stat.id().clone(),
        drink: drink.id().clone(),
        pause: pause.id().clone(),
        remind_now: remind_now.id().clone(),
        reset: reset.id().clone(),
        interval_30: i30.id().clone(),
        interval_45: i45.id().clone(),
        interval_60: i60.id().clone(),
        interval_90: i90.id().clone(),
        workdays_only: workdays_only.id().clone(),
        autostart: autostart.id().clone(),
        open_config: open_config.id().clone(),
        quit: quit.id().clone(),
    };
    (menu, ids)
}

fn load_icon() -> Result<Icon> {
    let img = image::load_from_memory(ICON_BYTES)?.into_rgba8();
    let (w, h) = img.dimensions();
    Ok(Icon::from_rgba(img.into_raw(), w, h)?)
}

fn open_path(p: &std::path::Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &p.to_string_lossy()])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(p).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}

fn main() -> Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // tray-icon 的菜单事件是全局 channel，需要桥接到 tao 事件循环
    let proxy_menu = proxy.clone();
    MenuEvent::set_event_handler(Some(move |ev| {
        let _ = proxy_menu.send_event(UserEvent::Menu(ev));
    }));

    // 1 秒 tick
    let proxy_tick = proxy.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let _ = proxy_tick.send_event(UserEvent::Tick);
    });

    // 注意：TrayIcon 必须在事件循环开始 *之后* 创建（macOS 限制），
    // 但 Windows / Linux 上提前创建也 OK。这里在 run 闭包前创建，
    // 已知在 Windows 上工作良好。
    let mut app = App::new()?;

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Tick) => app.on_tick(),
            Event::UserEvent(UserEvent::Menu(ev)) => {
                let _ = app.on_menu(ev);
            }
            _ => {}
        }
    });
}
