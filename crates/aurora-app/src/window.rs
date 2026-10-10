//! Windows Native Desktop Window & GDI Renderer for KUNYIN Neo.
//!
//! 100% Pure Rust Native Win32 Desktop Window:
//! - Direct Win32 Event Loop (GetMessageW / DispatchMessageW)
//! - Double-Buffered GDI Surface for flicker-free 60 FPS presentation
//! - Apple HIG 4-zone Desktop Layout: Sidebar, Toolbar, Main Song List, Bottom PlayerBar
//! - Interactive Route Navigation, 44pt Tactile Play/Pause button, scrubbing timeline timer

#[cfg(windows)]
pub mod win32 {
    use std::ptr::{null, null_mut};
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
        CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect,
        InvalidateRect, RoundRect, SelectObject, SetBkMode, SetTextColor,
        UpdateWindow, DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
        HBRUSH, HDC, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW,
        KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW, SetTimer, ShowWindow,
        TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, MSG,
        SW_SHOW, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_PAINT,
        WM_SIZE, WM_TIMER, WNDCLASSW, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    use crate::components::PlayerBarView;
    use crate::state::AppRoute;
    use crate::KunyinApp;

    const DT_RIGHT: u32 = 2;

    #[inline(always)]
    const fn rgb(r: u8, g: u8, b: u8) -> u32 {
        (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
    }

    static APP_INSTANCE: Mutex<Option<KunyinApp>> = Mutex::new(None);

    const TIMER_ID_PLAYBACK: usize = 1001;

    // Helper: Wide string conversion
    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // Window Procedure
    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_ERASEBKGND => 1, // Avoid flicker; we handle entire background in double buffer
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                paint_window(hwnd, hdc);
                EndPaint(hwnd, &ps);
                0
            }
            WM_SIZE => {
                InvalidateRect(hwnd, null(), 0);
                0
            }
            WM_TIMER => {
                if wparam as usize == TIMER_ID_PLAYBACK {
                    if let Ok(mut guard) = APP_INSTANCE.lock() {
                        if let Some(app) = guard.as_mut() {
                            if app.state.snapshot().status == aurora_player::PlaybackStatus::Playing {
                                app.state.tick(100);

                                if app.state.desktop_lyric_visible {
                                    if let Some(active) = &app.state.active_lyric {
                                        let appearance = aurora_core::lyrics::compute_desktop_lyric_appearance(
                                            app.state.desktop_lyric_opacity as f64,
                                            100.0,
                                            600,
                                            0.72,
                                            2,
                                        );
                                        let frame = aurora_core::lyrics::render_desktop_lyric_frame(
                                            &active.text,
                                            active.next_text.as_deref(),
                                            active.progress,
                                            "#E8C083",
                                            "#8E8E93",
                                            &appearance,
                                            24,
                                            true,
                                        );
                                        crate::desktop_lyric_window::win32::refresh_desktop_lyric(frame);
                                    }
                                }

                                InvalidateRect(hwnd, null(), 0);
                            }
                        }
                    }
                }
                0
            }
            WM_KEYDOWN => {
                // Space = Play/Pause
                if wparam == 0x20 {
                    if let Ok(mut guard) = APP_INSTANCE.lock() {
                        if let Some(app) = guard.as_mut() {
                            app.state.toggle_play_pause();
                            InvalidateRect(hwnd, null(), 0);
                        }
                    }
                }
                0
            }
            WM_LBUTTONDOWN => {
                let x = (lparam & 0xFFFF) as i32;
                let y = ((lparam >> 16) & 0xFFFF) as i32;
                handle_click(hwnd, x, y);
                0
            }
            WM_DESTROY => {
                KillTimer(hwnd, TIMER_ID_PLAYBACK);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn handle_click(hwnd: HWND, x: i32, y: i32) {
        let mut rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rect);
        let win_width = rect.right;
        let win_height = rect.bottom;

        if let Ok(mut guard) = APP_INSTANCE.lock() {
            if let Some(app) = guard.as_mut() {
                // Toolbar Desktop Lyric Pill toggle (x: win_width - 400 .. win_width - 275, y: 12..40)
                if x >= win_width - 400 && x <= win_width - 275 && y >= 12 && y <= 40 {
                    let active = app.state.toggle_desktop_lyric();
                    if active {
                        crate::desktop_lyric_window::win32::show_desktop_lyric_window();
                    } else {
                        crate::desktop_lyric_window::win32::hide_desktop_lyric_window();
                    }
                    InvalidateRect(hwnd, null(), 0);
                    return;
                }

                // 1. Sidebar items (x: 0..220)
                if x < 220 {
                    let nav_routes = [
                        AppRoute::Discover,
                        AppRoute::Search,
                        AppRoute::Library,
                        AppRoute::Trial,
                        AppRoute::Favorites,
                        AppRoute::Lyrics,
                        AppRoute::Settings,
                    ];
                    let start_y = 70;
                    let item_height = 42;
                    for (i, route) in nav_routes.into_iter().enumerate() {
                        let item_top = start_y + (i as i32) * item_height;
                        if y >= item_top && y < item_top + item_height {
                            app.state.navigate_to(route);
                            InvalidateRect(hwnd, null(), 0);
                            return;
                        }
                    }
                }

                // 2. Bottom PlayerBar (y: win_height - 80 .. win_height)
                if y >= win_height - 80 {
                    // Play/Pause button center: x = rect.right / 2, y = win_height - 52, radius = 22
                    let center_x = rect.right / 2;
                    let center_y = win_height - 52;
                    let dist_sq = (x - center_x) * (x - center_x) + (y - center_y) * (y - center_y);
                    if dist_sq <= 22 * 22 {
                        app.state.toggle_play_pause();
                        InvalidateRect(hwnd, null(), 0);
                        return;
                    }

                    // Progress bar click: x from (center_x - 180) to (center_x + 180)
                    let bar_left = center_x - 180;
                    let bar_right = center_x + 180;
                    if x >= bar_left && x <= bar_right && y >= win_height - 28 && y <= win_height - 8 {
                        let frac = (x - bar_left) as f32 / (bar_right - bar_left) as f32;
                        let duration = app.state.snapshot().duration_ms.max(180_000);
                        let target_ms = (frac * duration as f32) as u64;
                        app.state.seek_to(target_ms);
                        InvalidateRect(hwnd, null(), 0);
                        return;
                    }
                }

                // 3. Main song rows (x >= 230, y >= 110 .. win_height - 90)
                if x >= 230 && y >= 110 && y < win_height - 90 {
                    let row_idx = (y - 110) / 48;
                    let songs = get_sample_tracks();
                    if let Some(song) = songs.get(row_idx as usize) {
                        app.state.play_track(song.clone());
                        InvalidateRect(hwnd, null(), 0);
                        return;
                    }
                }
            }
        }
    }

    fn get_sample_tracks() -> Vec<crate::state::AppTrack> {
        vec![
            crate::state::AppTrack {
                id: 1,
                source: "local".to_string(),
                title: "晴天 (Acoustic Hi-Res)".to_string(),
                artist: "周杰伦".to_string(),
                album: "叶惠美".to_string(),
                duration_ms: 269_000,
                cover_url: None,
                file_path: None,
            },
            crate::state::AppTrack {
                id: 2,
                source: "local".to_string(),
                title: "起风了 (Live Master)".to_string(),
                artist: "买辣椒也用券".to_string(),
                album: "起风了".to_string(),
                duration_ms: 325_000,
                cover_url: None,
                file_path: None,
            },
            crate::state::AppTrack {
                id: 3,
                source: "local".to_string(),
                title: "海底 (24bit FLAC Studio)".to_string(),
                artist: "一支榴莲".to_string(),
                album: "独角兽".to_string(),
                duration_ms: 251_000,
                cover_url: None,
                file_path: None,
            },
            crate::state::AppTrack {
                id: 4,
                source: "local".to_string(),
                title: "慢冷 (Audiophile Edition)".to_string(),
                artist: "梁静茹".to_string(),
                album: "我好吗? 太阳如常升起".to_string(),
                duration_ms: 279_000,
                cover_url: None,
                file_path: None,
            },
            crate::state::AppTrack {
                id: 5,
                source: "local".to_string(),
                title: "如愿 (WASAPI Bit-Perfect)".to_string(),
                artist: "王菲".to_string(),
                album: "我和我的父辈 电影主题歌".to_string(),
                duration_ms: 298_000,
                cover_url: None,
                file_path: None,
            },
        ]
    }

    unsafe fn paint_window(hwnd: HWND, hdc: HDC) {
        let mut rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rect);
        let width = rect.right;
        let height = rect.bottom;
        if width <= 0 || height <= 0 {
            return;
        }

        // Create Memory DC & Bitmap for double-buffering
        let mem_dc = CreateCompatibleDC(hdc);
        let mem_bitmap = CreateCompatibleBitmap(hdc, width, height);
        let old_bitmap = SelectObject(mem_dc, mem_bitmap);

        // Fonts
        let font_title = CreateFontW(
            -20, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0,
            to_wide("Microsoft YaHei UI").as_ptr(),
        );
        let font_body = CreateFontW(
            -13, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
            to_wide("Microsoft YaHei UI").as_ptr(),
        );
        let font_body_bold = CreateFontW(
            -13, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0,
            to_wide("Microsoft YaHei UI").as_ptr(),
        );
        let font_small = CreateFontW(
            -11, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
            to_wide("Microsoft YaHei UI").as_ptr(),
        );

        let guard = APP_INSTANCE.lock().ok();
        let app = guard.as_ref().and_then(|g| g.as_ref());

        // 1. Overall Background: Apple Dark #0F0F12
        let bg_brush = CreateSolidBrush(rgb(15, 15, 18));
        FillRect(mem_dc, &rect, bg_brush);
        DeleteObject(bg_brush);

        // 2. Sidebar (Width: 220px, Height: height - 80)
        let sidebar_rect = RECT {
            left: 0,
            top: 0,
            right: 220,
            bottom: height - 80,
        };
        let sidebar_brush = CreateSolidBrush(rgb(22, 22, 26));
        FillRect(mem_dc, &sidebar_rect, sidebar_brush);
        DeleteObject(sidebar_brush);

        // Sidebar 1px Right Divider
        let div_brush = CreateSolidBrush(rgb(38, 38, 44));
        let div_rect = RECT {
            left: 219,
            top: 0,
            right: 220,
            bottom: height - 80,
        };
        FillRect(mem_dc, &div_rect, div_brush);
        DeleteObject(div_brush);

        // Sidebar Title
        SelectObject(mem_dc, font_title);
        SetBkMode(mem_dc, TRANSPARENT as i32);
        SetTextColor(mem_dc, rgb(255, 255, 255));
        let title_text = to_wide("昆音 KUNYIN");
        let mut title_rect = RECT {
            left: 24,
            top: 20,
            right: 210,
            bottom: 50,
        };
        DrawTextW(
            mem_dc,
            title_text.as_ptr(),
            -1,
            &mut title_rect,
            DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
        );

        // Sidebar Navigation Items
        let current_route = app
            .map(|a| a.state.current_route.clone())
            .unwrap_or(AppRoute::Discover);
        let nav_items = [
            ("发现音乐", AppRoute::Discover),
            ("音乐搜索", AppRoute::Search),
            ("本地曲库", AppRoute::Library),
            ("试听列表", AppRoute::Trial),
            ("我的收藏", AppRoute::Favorites),
            ("歌词视窗", AppRoute::Lyrics),
            ("音频设置", AppRoute::Settings),
        ];

        let start_y = 70;
        let item_height = 42;
        for (i, (title, route)) in nav_items.into_iter().enumerate() {
            let item_top = start_y + (i as i32) * item_height;
            let is_active = route == current_route;

            if is_active {
                // Active Capsule selection pill
                let pill_brush = CreateSolidBrush(rgb(28, 54, 90));
                let pill_rect = RECT {
                    left: 14,
                    top: item_top + 3,
                    right: 206,
                    bottom: item_top + 35,
                };
                let rgn = RoundRect(
                    mem_dc,
                    pill_rect.left,
                    pill_rect.top,
                    pill_rect.right,
                    pill_rect.bottom,
                    14,
                    14,
                );
                let _ = rgn;
                FillRect(mem_dc, &pill_rect, pill_brush);
                DeleteObject(pill_brush);

                SelectObject(mem_dc, font_body_bold);
                SetTextColor(mem_dc, rgb(10, 132, 255)); // Apple Blue
            } else {
                SelectObject(mem_dc, font_body);
                SetTextColor(mem_dc, rgb(160, 160, 168)); // Muted Text
            }

            let mut text_rect = RECT {
                left: 28,
                top: item_top,
                right: 200,
                bottom: item_top + item_height,
            };
            let text_wide = to_wide(title);
            DrawTextW(
                mem_dc,
                text_wide.as_ptr(),
                -1,
                &mut text_rect,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }

        // 3. Top Toolbar (x: 220 .. width, height: 52)
        let toolbar_rect = RECT {
            left: 220,
            top: 0,
            right: width,
            bottom: 52,
        };
        let toolbar_brush = CreateSolidBrush(rgb(20, 20, 24));
        FillRect(mem_dc, &toolbar_rect, toolbar_brush);
        DeleteObject(toolbar_brush);

        // Toolbar bottom 1px divider
        let tb_div_brush = CreateSolidBrush(rgb(38, 38, 44));
        let tb_div_rect = RECT {
            left: 220,
            top: 51,
            right: width,
            bottom: 52,
        };
        FillRect(mem_dc, &tb_div_rect, tb_div_brush);
        DeleteObject(tb_div_brush);

        // Search Pill in Toolbar
        let search_brush = CreateSolidBrush(rgb(32, 32, 38));
        let search_rect = RECT {
            left: 240,
            top: 10,
            right: 520.min(width - 240),
            bottom: 42,
        };
        FillRect(mem_dc, &search_rect, search_brush);
        DeleteObject(search_brush);

        SelectObject(mem_dc, font_body);
        SetTextColor(mem_dc, rgb(120, 120, 130));
        let mut search_text_rect = RECT {
            left: 254,
            top: 10,
            right: 510.min(width - 240),
            bottom: 42,
        };
        let search_placeholder = to_wide("搜索音乐、歌手、专辑...");
        DrawTextW(
            mem_dc,
            search_placeholder.as_ptr(),
            -1,
            &mut search_text_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        // Desktop Lyric Toggle Pill in Toolbar (x: width - 420 .. width - 280)
        let dl_active = app.map(|a| a.state.desktop_lyric_visible).unwrap_or(false);
        let dl_brush = CreateSolidBrush(if dl_active { rgb(28, 54, 90) } else { rgb(32, 32, 38) });
        let dl_rect = RECT {
            left: width - 420,
            top: 12,
            right: width - 280,
            bottom: 40,
        };
        FillRect(mem_dc, &dl_rect, dl_brush);
        DeleteObject(dl_brush);

        SelectObject(mem_dc, font_small);
        SetTextColor(mem_dc, if dl_active { rgb(10, 132, 255) } else { rgb(160, 160, 170) });
        let mut dl_text_rect = dl_rect;
        let dl_text = to_wide(if dl_active { "★ 桌面歌词: 开启" } else { "☆ 桌面歌词: 关闭" });
        DrawTextW(
            mem_dc,
            dl_text.as_ptr(),
            -1,
            &mut dl_text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        // Engine Status Pill in Toolbar
        let badge_brush = CreateSolidBrush(rgb(22, 40, 32));
        let badge_rect = RECT {
            left: width - 260,
            top: 12,
            right: width - 20,
            bottom: 40,
        };
        FillRect(mem_dc, &badge_rect, badge_brush);
        DeleteObject(badge_brush);

        SelectObject(mem_dc, font_small);
        SetTextColor(mem_dc, rgb(52, 199, 89)); // Apple Green
        let mut badge_text_rect = badge_rect;
        let badge_text = to_wide("● WASAPI 独占直通 · 48kHz 24bit");
        DrawTextW(
            mem_dc,
            badge_text.as_ptr(),
            -1,
            &mut badge_text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        // 4. Main View (x: 220 .. width, y: 52 .. height - 80)
        SelectObject(mem_dc, font_title);
        SetTextColor(mem_dc, rgb(255, 255, 255));
        let view_title = match current_route {
            AppRoute::Discover => "发现精选曲目",
            AppRoute::Search => "聚合多源音乐搜索",
            AppRoute::Library => "本地无损曲库",
            AppRoute::Trial => "试听列表",
            AppRoute::Favorites => "我的收藏",
            AppRoute::Lyrics => "实时逐字歌词视窗",
            AppRoute::Settings => "音频引擎与均衡器设置",
            _ => "音乐列表",
        };
        let mut v_title_rect = RECT {
            left: 240,
            top: 68,
            right: width - 40,
            bottom: 100,
        };
        let v_title_wide = to_wide(view_title);
        DrawTextW(
            mem_dc,
            v_title_wide.as_ptr(),
            -1,
            &mut v_title_rect,
            DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
        );

        if current_route == AppRoute::Lyrics {
            let default_lrc = "[00:00.00]昆音 (KUNYIN Neo) - 纯原生无损音频引擎\n[00:05.00]零 WebView · 零 Electron · 纯 Rust 打造\n[00:10.00]Apple Human Interface Guidelines 经典美学\n[00:15.00]10 段双二阶 IIR 均衡器与 WASAPI 独占硬件输出\n[00:20.00]高精度毫秒级时间轴与逐字卡拉OK平滑推进\n[00:25.00]原生 Win32 分层透明置顶桌面歌词视窗\n[00:30.00]极低内存常驻 · 零 CPU 浪费 · 极速启动";
            let lines = if let Some(app_ref) = app {
                if app_ref.state.lyrics_lines.is_empty() {
                    aurora_core::lyrics::parse_lrc(default_lrc)
                } else {
                    app_ref.state.lyrics_lines.clone()
                }
            } else {
                aurora_core::lyrics::parse_lrc(default_lrc)
            };

            let cur_pos = app.map(|a| a.state.snapshot().position_ms).unwrap_or(0);
            let model = crate::components::LyricViewModel::new(lines);
            let viewport = model.build_viewport(cur_pos, 7);

            let l_top = 130;
            let l_height = 56;
            for (idx, item) in viewport.into_iter().enumerate() {
                let row_y = l_top + (idx as i32) * l_height;
                if row_y + l_height > height - 80 {
                    break;
                }

                if item.is_active {
                    SelectObject(mem_dc, font_title);
                    SetTextColor(mem_dc, rgb(232, 192, 131)); // Gold active accent #E8C083
                } else {
                    SelectObject(mem_dc, font_body);
                    SetTextColor(mem_dc, rgb(142, 142, 147)); // Secondary muted
                }

                let mut r_line = RECT {
                    left: 240,
                    top: row_y,
                    right: width - 40,
                    bottom: row_y + l_height,
                };
                DrawTextW(
                    mem_dc,
                    to_wide(&item.text).as_ptr(),
                    -1,
                    &mut r_line,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        } else {
            // Table Column Headers
            SelectObject(mem_dc, font_small);
            SetTextColor(mem_dc, rgb(130, 130, 140));
            let col_y = 108;
            let mut c1 = RECT {
                left: 240,
                top: col_y,
                right: 270,
                bottom: col_y + 20,
            };
            let mut c2 = RECT {
                left: 270,
                top: col_y,
                right: 560,
                bottom: col_y + 20,
            };
            let mut c3 = RECT {
                left: 560,
                top: col_y,
                right: 740,
                bottom: col_y + 20,
            };
            let mut c4 = RECT {
                left: 740,
                top: col_y,
                right: 920,
                bottom: col_y + 20,
            };
            let mut c5 = RECT {
                left: width - 100,
                top: col_y,
                right: width - 40,
                bottom: col_y + 20,
            };

            DrawTextW(mem_dc, to_wide("#").as_ptr(), -1, &mut c1, DT_LEFT | DT_SINGLELINE);
            DrawTextW(mem_dc, to_wide("曲目标题").as_ptr(), -1, &mut c2, DT_LEFT | DT_SINGLELINE);
            DrawTextW(mem_dc, to_wide("歌手").as_ptr(), -1, &mut c3, DT_LEFT | DT_SINGLELINE);
            DrawTextW(mem_dc, to_wide("专辑").as_ptr(), -1, &mut c4, DT_LEFT | DT_SINGLELINE);
            DrawTextW(mem_dc, to_wide("时长").as_ptr(), -1, &mut c5, DT_RIGHT | DT_SINGLELINE);

            // Sample Songs Render
            let songs = get_sample_tracks();
            let playing_title = app
                .and_then(|a| a.state.current_track.as_ref())
                .map(|t| t.title.as_str())
                .unwrap_or("");

        for (idx, song) in songs.iter().enumerate() {
            let row_top = 136 + (idx as i32) * 48;
            if row_top + 48 > height - 80 {
                break;
            }

            let is_this_playing = song.title == playing_title;

            // Row highlight if playing
            if is_this_playing {
                let row_bg = CreateSolidBrush(rgb(22, 38, 60));
                let r_rect = RECT {
                    left: 230,
                    top: row_top,
                    right: width - 20,
                    bottom: row_top + 44,
                };
                FillRect(mem_dc, &r_rect, row_bg);
                DeleteObject(row_bg);
            }

            // Index
            SelectObject(mem_dc, font_body);
            SetTextColor(
                mem_dc,
                if is_this_playing {
                    rgb(10, 132, 255)
                } else {
                    rgb(120, 120, 130)
                },
            );
            let mut r_idx = RECT {
                left: 240,
                top: row_top,
                right: 265,
                bottom: row_top + 44,
            };
            DrawTextW(
                mem_dc,
                to_wide(&format!("{:02}", idx + 1)).as_ptr(),
                -1,
                &mut r_idx,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE,
            );

            // Title
            SelectObject(mem_dc, if is_this_playing { font_body_bold } else { font_body });
            SetTextColor(
                mem_dc,
                if is_this_playing {
                    rgb(10, 132, 255)
                } else {
                    rgb(245, 245, 247)
                },
            );
            let mut r_title = RECT {
                left: 270,
                top: row_top,
                right: 550,
                bottom: row_top + 44,
            };
            DrawTextW(
                mem_dc,
                to_wide(&song.title).as_ptr(),
                -1,
                &mut r_title,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            // Artist
            SelectObject(mem_dc, font_body);
            SetTextColor(mem_dc, rgb(160, 160, 170));
            let mut r_art = RECT {
                left: 560,
                top: row_top,
                right: 730,
                bottom: row_top + 44,
            };
            DrawTextW(
                mem_dc,
                to_wide(&song.artist).as_ptr(),
                -1,
                &mut r_art,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            // Album
            let mut r_alb = RECT {
                left: 740,
                top: row_top,
                right: 910,
                bottom: row_top + 44,
            };
            DrawTextW(
                mem_dc,
                to_wide(&song.album).as_ptr(),
                -1,
                &mut r_alb,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            // Duration
            let mut r_dur = RECT {
                left: width - 100,
                top: row_top,
                right: width - 40,
                bottom: row_top + 44,
            };
            DrawTextW(
                mem_dc,
                to_wide(&PlayerBarView::format_duration(song.duration_ms)).as_ptr(),
                -1,
                &mut r_dur,
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
            );
        }
        }

        // 5. Bottom PlayerBar (Height: 80px)
        let player_bar_rect = RECT {
            left: 0,
            top: height - 80,
            right: width,
            bottom: height,
        };
        let p_brush = CreateSolidBrush(rgb(24, 24, 28));
        FillRect(mem_dc, &player_bar_rect, p_brush);
        DeleteObject(p_brush);

        // PlayerBar top 1px border highlight
        let pb_div_brush = CreateSolidBrush(rgb(48, 48, 56));
        let pb_div_rect = RECT {
            left: 0,
            top: height - 80,
            right: width,
            bottom: height - 79,
        };
        FillRect(mem_dc, &pb_div_rect, pb_div_brush);
        DeleteObject(pb_div_brush);

        // Album Art Cover Squircle (52 x 52)
        let cover_rect = RECT {
            left: 18,
            top: height - 66,
            right: 70,
            bottom: height - 14,
        };
        let cover_brush = CreateSolidBrush(rgb(38, 42, 54));
        FillRect(mem_dc, &cover_rect, cover_brush);
        DeleteObject(cover_brush);

        SelectObject(mem_dc, font_title);
        SetTextColor(mem_dc, rgb(10, 132, 255));
        let mut note_rect = cover_rect;
        DrawTextW(mem_dc, to_wide("♫").as_ptr(), -1, &mut note_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

        // Currently Playing Track Details
        let curr_track = app.and_then(|a| a.state.current_track.as_ref());
        let current_title = curr_track.map(|t| t.title.as_str()).unwrap_or("昆音 - 原生极光引擎就绪");
        let current_artist = curr_track.map(|t| t.artist.as_str()).unwrap_or("Symphonia + WASAPI");

        SelectObject(mem_dc, font_body_bold);
        SetTextColor(mem_dc, rgb(255, 255, 255));
        let mut t_name_rect = RECT {
            left: 82,
            top: height - 60,
            right: 320,
            bottom: height - 40,
        };
        DrawTextW(mem_dc, to_wide(current_title).as_ptr(), -1, &mut t_name_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

        SelectObject(mem_dc, font_small);
        SetTextColor(mem_dc, rgb(140, 140, 150));
        let mut a_name_rect = RECT {
            left: 82,
            top: height - 38,
            right: 320,
            bottom: height - 20,
        };
        DrawTextW(mem_dc, to_wide(current_artist).as_ptr(), -1, &mut a_name_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

        // Playback Controls (Center)
        let center_x = width / 2;
        let is_playing = app
            .map(|a| a.state.snapshot().status == aurora_player::PlaybackStatus::Playing)
            .unwrap_or(false);

        // Previous button (◀◀)
        SelectObject(mem_dc, font_body);
        SetTextColor(mem_dc, rgb(200, 200, 210));
        let mut prev_rect = RECT {
            left: center_x - 70,
            top: height - 68,
            right: center_x - 38,
            bottom: height - 36,
        };
        DrawTextW(mem_dc, to_wide("◀◀").as_ptr(), -1, &mut prev_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

        // 44pt Play/Pause tactile circular button (Apple HIG 44px)
        let play_btn_brush = CreateSolidBrush(rgb(10, 132, 255)); // Apple Blue
        let btn_rect = RECT {
            left: center_x - 22,
            top: height - 74,
            right: center_x + 22,
            bottom: height - 30,
        };
        FillRect(mem_dc, &btn_rect, play_btn_brush);
        DeleteObject(play_btn_brush);

        SelectObject(mem_dc, font_title);
        SetTextColor(mem_dc, rgb(255, 255, 255));
        let mut p_icon_rect = btn_rect;
        let icon_str = if is_playing { "❚❚" } else { "▶" };
        DrawTextW(mem_dc, to_wide(icon_str).as_ptr(), -1, &mut p_icon_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

        // Next button (▶▶)
        SelectObject(mem_dc, font_body);
        SetTextColor(mem_dc, rgb(200, 200, 210));
        let mut next_rect = RECT {
            left: center_x + 38,
            top: height - 68,
            right: center_x + 70,
            bottom: height - 36,
        };
        DrawTextW(mem_dc, to_wide("▶▶").as_ptr(), -1, &mut next_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

        // Progress Bar & Time
        let snap = app.map(|a| a.state.snapshot()).unwrap_or_default();
        let pos_ms = snap.position_ms;
        let dur_ms = snap.duration_ms.max(180_000);
        let progress_frac = (pos_ms as f32 / dur_ms as f32).clamp(0.0, 1.0);

        // Time strings
        SelectObject(mem_dc, font_small);
        SetTextColor(mem_dc, rgb(140, 140, 150));
        let mut pos_text_rect = RECT {
            left: center_x - 230,
            top: height - 25,
            right: center_x - 190,
            bottom: height - 9,
        };
        DrawTextW(
            mem_dc,
            to_wide(&PlayerBarView::format_duration(pos_ms)).as_ptr(),
            -1,
            &mut pos_text_rect,
            DT_RIGHT | DT_SINGLELINE,
        );

        let mut dur_text_rect = RECT {
            left: center_x + 190,
            top: height - 25,
            right: center_x + 230,
            bottom: height - 9,
        };
        DrawTextW(
            mem_dc,
            to_wide(&PlayerBarView::format_duration(dur_ms)).as_ptr(),
            -1,
            &mut dur_text_rect,
            DT_LEFT | DT_SINGLELINE,
        );

        // Progress track (background bar)
        let bar_left = center_x - 180;
        let bar_right = center_x + 180;
        let bar_y = height - 17;
        let bar_bg = CreateSolidBrush(rgb(50, 50, 60));
        let bar_full_rect = RECT {
            left: bar_left,
            top: bar_y,
            right: bar_right,
            bottom: bar_y + 4,
        };
        FillRect(mem_dc, &bar_full_rect, bar_bg);
        DeleteObject(bar_bg);

        // Progress fill (Apple Blue active bar)
        let fill_right = bar_left + ((bar_right - bar_left) as f32 * progress_frac) as i32;
        let bar_fill = CreateSolidBrush(rgb(10, 132, 255));
        let bar_filled_rect = RECT {
            left: bar_left,
            top: bar_y,
            right: fill_right,
            bottom: bar_y + 4,
        };
        FillRect(mem_dc, &bar_filled_rect, bar_fill);
        DeleteObject(bar_fill);

        // Right Volume & Bit-Perfect indicator
        SelectObject(mem_dc, font_small);
        SetTextColor(mem_dc, rgb(160, 160, 170));
        let mut vol_text_rect = RECT {
            left: width - 160,
            top: height - 52,
            right: width - 20,
            bottom: height - 28,
        };
        DrawTextW(
            mem_dc,
            to_wide("音量 85% · 独占直通").as_ptr(),
            -1,
            &mut vol_text_rect,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
        );

        // Transfer Double-Buffered content to screen
        BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

        // Clean up GDI objects
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(mem_bitmap);
        DeleteDC(mem_dc);
        DeleteObject(font_title);
        DeleteObject(font_body);
        DeleteObject(font_body_bold);
        DeleteObject(font_small);
    }

    pub fn run_desktop_window(app: KunyinApp) -> Result<(), String> {
        let instance = unsafe { GetModuleHandleW(null_mut()) };
        let class_name = to_wide("KunyinNeoNativeWindowClass");

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
            hbrBackground: 0 as HBRUSH,
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
        };

        unsafe {
            RegisterClassW(&wc);
        }

        // Store App state globally for window procedure
        if let Ok(mut guard) = APP_INSTANCE.lock() {
            *guard = Some(app);
        }

        let title = to_wide("昆音 (Kunyin Neo) - Apple Design 原生版");
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_APPWINDOW,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1120,
                720,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            )
        };

        if hwnd == null_mut() {
            return Err("Failed to create Win32 native window".to_string());
        }

        // Set playback timeline refresh timer (every 100ms)
        unsafe {
            SetTimer(hwnd, TIMER_ID_PLAYBACK, 100, None);
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
            crate::desktop_lyric_window::win32::init_desktop_lyric_window(
                crate::desktop_lyric_window::DesktopLyricWindowConfig::default(),
            );
        }

        // Standard Windows Message Pump
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        unsafe {
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        Ok(())
    }
}

#[cfg(not(windows))]
pub mod win32 {
    use crate::KunyinApp;
    pub fn run_desktop_window(_app: KunyinApp) -> Result<(), String> {
        Err("Desktop window only supported on Windows host".to_string())
    }
}
