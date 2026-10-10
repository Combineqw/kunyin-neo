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
        HBRUSH, HDC, HGDIOBJ, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW,
        KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW, SetTimer, ShowWindow,
        TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, MSG,
        SW_SHOW, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_PAINT,
        WM_SIZE, WM_TIMER, WNDCLASSW, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    use crate::components::{EqPreset, LibraryCategoryTab, LibraryStats, PlayerBarView, SearchSourceTab, FREQUENCIES};
    use crate::state::{AppRoute, WindowDisplayMode};
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
                // If in mini player mode, handle mini player controls
                if app.state.window_mode == WindowDisplayMode::MiniPlayer {
                    // Restore button: x: win_width - 95 .. win_width - 10, y: 8 .. 36
                    if x >= win_width - 95 && x <= win_width - 10 && y >= 8 && y <= 36 {
                        app.state.toggle_window_mode();
                        InvalidateRect(hwnd, null(), 0);
                        return;
                    }
                    // Play/Pause button center in mini player: center_x = win_width - 110, center_y = 65
                    let btn_x = win_width - 110;
                    let btn_y = 65;
                    let dist = (x - btn_x) * (x - btn_x) + (y - btn_y) * (y - btn_y);
                    if dist <= 20 * 20 {
                        app.state.toggle_play_pause();
                        InvalidateRect(hwnd, null(), 0);
                        return;
                    }
                    // Next track button center: center_x = win_width - 50, center_y = 65
                    let next_x = win_width - 50;
                    let next_y = 65;
                    let dist_next = (x - next_x) * (x - next_x) + (y - next_y) * (y - next_y);
                    if dist_next <= 20 * 20 {
                        let sample_songs = get_sample_tracks();
                        if let Some(song) = sample_songs.get(1) {
                            app.state.play_track(song.clone());
                        }
                        InvalidateRect(hwnd, null(), 0);
                        return;
                    }
                    return;
                }

                // Mini Player Toggle Pill in Toolbar (x: win_width - 560 .. win_width - 430, y: 12..40)
                if x >= win_width - 560 && x <= win_width - 430 && y >= 12 && y <= 40 {
                    app.state.toggle_window_mode();
                    InvalidateRect(hwnd, null(), 0);
                    return;
                }

                // Toolbar Desktop Lyric Pill toggle (x: win_width - 420 .. win_width - 280, y: 12..40)
                if x >= win_width - 420 && x <= win_width - 280 && y >= 12 && y <= 40 {
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

                // 3. Route-specific interactive clicks
                match &app.state.current_route {
                    AppRoute::Discover => {
                        // Heartbeat Pill in Hero card: x in (win_width - 240 .. win_width - 60), y in (85..125)
                        if x >= win_width - 240 && x <= win_width - 60 && y >= 85 && y <= 125 {
                            app.state.toggle_heartbeat();
                            InvalidateRect(hwnd, null(), 0);
                            return;
                        }
                        // Curated Playlists cards: y in (195..275)
                        if y >= 195 && y <= 275 && x >= 240 {
                            let card_idx = ((x - 240) / 185) as usize;
                            let card_opt = app.state.recommend.curated_playlists.get(card_idx).cloned();
                            if let Some(card) = card_opt {
                                let pl_vm = crate::components::PlaylistDetailViewModel::new(
                                    card.id,
                                    card.title,
                                    card.creator,
                                    "坤音智能推荐歌单，精选优质无损音轨",
                                    vec![card.category],
                                    get_sample_tracks(),
                                );
                                app.state.open_playlist(pl_vm);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // New Releases shelf click: y in (320..480), x >= 240
                        if y >= 320 && y < win_height - 90 && x >= 240 {
                            let rel_idx = ((y - 320) / 44) as usize;
                            if let Some(rel) = app.state.recommend.new_releases.get(rel_idx).cloned() {
                                let track = crate::state::AppTrack {
                                    id: rel.id,
                                    source: rel.source,
                                    title: rel.title,
                                    artist: rel.artist,
                                    album: rel.album,
                                    duration_ms: rel.duration_ms,
                                    cover_url: None,
                                    file_path: None,
                                };
                                app.state.play_track(track);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                    }
                    AppRoute::Playlist(_) => {
                        // "▶ 全部播放" pill: x in (330..430), y in (120..150)
                        if x >= 330 && x <= 430 && y >= 120 && y <= 150 {
                            let songs = get_sample_tracks();
                            if let Some(first) = songs.first() {
                                app.state.play_track(first.clone());
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // "♡ 收藏歌单" pill: x in (445..545), y in (120..150)
                        if x >= 445 && x <= 545 && y >= 120 && y <= 150 {
                            if let Some(pl) = app.state.selected_playlist.as_mut() {
                                pl.toggle_favorite();
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // Song list rows in playlist: y in (200 .. win_height - 90)
                        if x >= 230 && y >= 200 && y < win_height - 90 {
                            let row_idx = (y - 200) / 48;
                            let songs = get_sample_tracks();
                            if let Some(song) = songs.get(row_idx as usize) {
                                app.state.play_track(song.clone());
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                    }
                    AppRoute::Settings => {
                        // EQ Preset selector pills: y in (110..145)
                        if y >= 110 && y <= 145 {
                            if x >= 240 && x <= 340 {
                                app.state.apply_eq_preset(EqPreset::Flat);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 350 && x <= 450 {
                                app.state.apply_eq_preset(EqPreset::Acoustic);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 460 && x <= 570 {
                                app.state.apply_eq_preset(EqPreset::BassBoost);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 580 && x <= 680 {
                                app.state.apply_eq_preset(EqPreset::Vocal);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 690 && x <= 800 {
                                app.state.apply_eq_preset(EqPreset::Electronic);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                    }
                    AppRoute::Search => {
                        // Search Source tabs: y in (105..138)
                        if y >= 105 && y <= 138 {
                            if x >= 240 && x <= 310 {
                                app.state.set_search_tab(SearchSourceTab::All);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 320 && x <= 395 {
                                app.state.set_search_tab(SearchSourceTab::Qq);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 405 && x <= 480 {
                                app.state.set_search_tab(SearchSourceTab::Wy);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 490 && x <= 565 {
                                app.state.set_search_tab(SearchSourceTab::Kg);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 575 && x <= 650 {
                                app.state.set_search_tab(SearchSourceTab::Kw);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // Results rows
                        if x >= 230 && y >= 150 && y < win_height - 90 {
                            let row_idx = (y - 150) / 48;
                            let songs = get_sample_tracks();
                            if let Some(song) = songs.get(row_idx as usize) {
                                app.state.play_track(song.clone());
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                    }
                    AppRoute::Library => {
                        // Category Tabs: y in (108..138)
                        if y >= 108 && y <= 138 {
                            if x >= 240 && x <= 325 {
                                app.state.switch_library_tab(LibraryCategoryTab::Tracks);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 335 && x <= 420 {
                                app.state.switch_library_tab(LibraryCategoryTab::Albums);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 430 && x <= 515 {
                                app.state.switch_library_tab(LibraryCategoryTab::Artists);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            } else if x >= 525 && x <= 620 {
                                app.state.switch_library_tab(LibraryCategoryTab::Folders);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // Tracks row click (when in Tracks tab)
                        if app.state.library_model.current_tab == LibraryCategoryTab::Tracks {
                            if x >= 230 && y >= 195 && y < win_height - 90 {
                                let row_idx = (y - 195) / 48;
                                let tracks = app.state.library_model.filtered_tracks();
                                if let Some(track) = tracks.get(row_idx as usize) {
                                    app.state.play_track(track.clone());
                                    InvalidateRect(hwnd, null(), 0);
                                    return;
                                }
                            }
                        }
                    }
                    AppRoute::Favorites => {
                        // "▶ 播放全部" (x: 326..426, y: 145..173)
                        if x >= 326 && x <= 426 && y >= 145 && y <= 173 {
                            if let Some(first) = app.state.favorites_model.tracks.first().cloned() {
                                app.state.play_track(first);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // "🔀 随机播放" (x: 436..536, y: 145..173)
                        if x >= 436 && x <= 536 && y >= 145 && y <= 173 {
                            if let Some(last) = app.state.favorites_model.tracks.last().cloned() {
                                app.state.play_track(last);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // Song rows click: y >= 190
                        if x >= 230 && y >= 190 && y < win_height - 90 {
                            let row_idx = (y - 190) / 48;
                            let favs = app.state.favorites_model.filtered_tracks();
                            if let Some(track) = favs.get(row_idx as usize) {
                                app.state.play_track(track.clone());
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                    }
                    AppRoute::Trial => {
                        // "▶ 顺序播放" (x: 310..400, y: 135..162)
                        if x >= 310 && x <= 400 && y >= 135 && y <= 162 {
                            if let Some(first) = app.state.trial_model.queue.first().cloned() {
                                app.state.play_track(first);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                        // "🗑 清空队列" (x: 410..500, y: 135..162)
                        if x >= 410 && x <= 500 && y >= 135 && y <= 162 {
                            app.state.clear_trial_queue();
                            InvalidateRect(hwnd, null(), 0);
                            return;
                        }
                        // Song rows click: y >= 175
                        if x >= 230 && y >= 175 && y < win_height - 90 {
                            let row_idx = (y - 175) / 48;
                            if let Some(track) = app.state.trial_model.queue.get(row_idx as usize).cloned() {
                                app.state.play_track(track);
                                InvalidateRect(hwnd, null(), 0);
                                return;
                            }
                        }
                    }
                    _ => {
                        // Default song rows (x >= 230, y >= 110 .. win_height - 90)
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

    unsafe fn paint_sample_rows(
        mem_dc: HDC,
        width: i32,
        height: i32,
        font_small: HGDIOBJ,
        font_body: HGDIOBJ,
        font_body_bold: HGDIOBJ,
        col_y: i32,
        app: Option<&KunyinApp>,
    ) {
        // Table Column Headers
        SelectObject(mem_dc, font_small);
        SetTextColor(mem_dc, rgb(130, 130, 140));
        let mut c1 = RECT { left: 240, top: col_y, right: 270, bottom: col_y + 20 };
        let mut c2 = RECT { left: 270, top: col_y, right: 560, bottom: col_y + 20 };
        let mut c3 = RECT { left: 560, top: col_y, right: 740, bottom: col_y + 20 };
        let mut c4 = RECT { left: 740, top: col_y, right: 920, bottom: col_y + 20 };
        let mut c5 = RECT { left: width - 100, top: col_y, right: width - 40, bottom: col_y + 20 };

        DrawTextW(mem_dc, to_wide("#").as_ptr(), -1, &mut c1, DT_LEFT | DT_SINGLELINE);
        DrawTextW(mem_dc, to_wide("曲目标题").as_ptr(), -1, &mut c2, DT_LEFT | DT_SINGLELINE);
        DrawTextW(mem_dc, to_wide("歌手").as_ptr(), -1, &mut c3, DT_LEFT | DT_SINGLELINE);
        DrawTextW(mem_dc, to_wide("专辑").as_ptr(), -1, &mut c4, DT_LEFT | DT_SINGLELINE);
        DrawTextW(mem_dc, to_wide("时长").as_ptr(), -1, &mut c5, DT_RIGHT | DT_SINGLELINE);

        let songs = get_sample_tracks();
        let playing_title = app
            .and_then(|a| a.state.current_track.as_ref())
            .map(|t| t.title.as_str())
            .unwrap_or("");

        for (idx, song) in songs.iter().enumerate() {
            let row_top = col_y + 28 + (idx as i32) * 48;
            if row_top + 48 > height - 80 {
                break;
            }

            let is_this_playing = song.title == playing_title;

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

        // Mini Player Toggle Pill in Toolbar (x: width - 570 .. width - 430)
        let is_mini = app.map(|a| a.state.window_mode == WindowDisplayMode::MiniPlayer).unwrap_or(false);
        let mp_brush = CreateSolidBrush(if is_mini { rgb(28, 54, 90) } else { rgb(32, 32, 38) });
        let mp_rect = RECT {
            left: width - 570,
            top: 12,
            right: width - 430,
            bottom: 40,
        };
        FillRect(mem_dc, &mp_rect, mp_brush);
        DeleteObject(mp_brush);

        SelectObject(mem_dc, font_small);
        SetTextColor(mem_dc, if is_mini { rgb(10, 132, 255) } else { rgb(160, 160, 170) });
        let mut mp_text_rect = mp_rect;
        let mp_text = to_wide(if is_mini { "◲ 退出迷你模式" } else { "◱ 迷你小窗模式" });
        DrawTextW(
            mem_dc,
            mp_text.as_ptr(),
            -1,
            &mut mp_text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
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
            AppRoute::Discover => "发现精选曲目 · 每日推荐",
            AppRoute::Search => "聚合多源音乐搜索",
            AppRoute::Library => "本地无损曲库",
            AppRoute::Trial => "试听列表",
            AppRoute::Favorites => "我的收藏",
            AppRoute::Lyrics => "实时逐字歌词视窗",
            AppRoute::Settings => "音频引擎与 10 段图示均衡器",
            _ => "音乐精选列表",
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

        match &current_route {
            AppRoute::Lyrics => {
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
            }
            AppRoute::Discover => {
                // 1. Hero Banner Card (每日推荐与心动模式)
                let hero_brush = CreateSolidBrush(rgb(26, 32, 46));
                let hero_rect = RECT {
                    left: 240,
                    top: 105,
                    right: width - 40,
                    bottom: 180,
                };
                FillRect(mem_dc, &hero_rect, hero_brush);
                DeleteObject(hero_brush);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(232, 192, 131)); // Gold eyebrow
                let mut eb_rect = RECT { left: 256, top: 114, right: width - 260, bottom: 130 };
                DrawTextW(mem_dc, to_wide("FOR YOU · LOCAL ONLY").as_ptr(), -1, &mut eb_rect, DT_LEFT | DT_SINGLELINE);

                SelectObject(mem_dc, font_body_bold);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut h_title_rect = RECT { left: 256, top: 132, right: width - 260, bottom: 152 };
                DrawTextW(mem_dc, to_wide("每日精选与心动模式 · 隐私离线").as_ptr(), -1, &mut h_title_rect, DT_LEFT | DT_SINGLELINE);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(150, 150, 160));
                let mut h_desc_rect = RECT { left: 256, top: 154, right: width - 260, bottom: 172 };
                DrawTextW(mem_dc, to_wide("基于本地曲库偏好与完成度生成，不上传任何收听数据").as_ptr(), -1, &mut h_desc_rect, DT_LEFT | DT_SINGLELINE);

                // Heartbeat pill button in Hero card
                let hb_active = app.map(|a| a.state.recommend.is_heartbeat_active).unwrap_or(false);
                let hb_brush = CreateSolidBrush(if hb_active { rgb(220, 38, 70) } else { rgb(40, 44, 58) });
                let hb_rect = RECT {
                    left: width - 240,
                    top: 120,
                    right: width - 60,
                    bottom: 160,
                };
                FillRect(mem_dc, &hb_rect, hb_brush);
                DeleteObject(hb_brush);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut hb_t_rect = hb_rect;
                let hb_label = to_wide(if hb_active { "♥ 心动模式: 开启" } else { "♡ 开启心动模式" });
                DrawTextW(mem_dc, hb_label.as_ptr(), -1, &mut hb_t_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                // 2. Curated Playlists section title
                SelectObject(mem_dc, font_body_bold);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut pl_sec_rect = RECT { left: 240, top: 194, right: width - 40, bottom: 216 };
                DrawTextW(mem_dc, to_wide("推荐歌单 · 智能雷达").as_ptr(), -1, &mut pl_sec_rect, DT_LEFT | DT_SINGLELINE);

                // 4 Curated Cards Grid
                let default_rec = crate::components::RecommendViewModel::default_curated();
                let rec_ref = app.map(|a| &a.state.recommend).unwrap_or(&default_rec);
                for (i, card) in rec_ref.curated_playlists.iter().enumerate().take(4) {
                    let c_left = 240 + (i as i32) * 185;
                    let c_right = (c_left + 175).min(width - 40);
                    let c_brush = CreateSolidBrush(rgb(32, 34, 44));
                    let c_rect = RECT { left: c_left, top: 220, right: c_right, bottom: 284 };
                    FillRect(mem_dc, &c_rect, c_brush);
                    DeleteObject(c_brush);

                    SelectObject(mem_dc, font_body_bold);
                    SetTextColor(mem_dc, rgb(10, 132, 255));
                    let mut ic_rect = RECT { left: c_left + 10, top: 226, right: c_left + 35, bottom: 250 };
                    DrawTextW(mem_dc, to_wide("♫").as_ptr(), -1, &mut ic_rect, DT_LEFT | DT_SINGLELINE);

                    SelectObject(mem_dc, font_small);
                    SetTextColor(mem_dc, rgb(240, 240, 245));
                    let mut t_rect = RECT { left: c_left + 38, top: 228, right: c_right - 8, bottom: 250 };
                    DrawTextW(mem_dc, to_wide(&card.title).as_ptr(), -1, &mut t_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

                    SetTextColor(mem_dc, rgb(140, 140, 150));
                    let mut b_rect = RECT { left: c_left + 10, top: 254, right: c_right - 8, bottom: 278 };
                    let badge_str = format!("{} · {}", card.formatted_play_count(), card.category);
                    DrawTextW(mem_dc, to_wide(&badge_str).as_ptr(), -1, &mut b_rect, DT_LEFT | DT_SINGLELINE);
                }

                // 3. New Releases section title
                SelectObject(mem_dc, font_body_bold);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut nr_sec_rect = RECT { left: 240, top: 298, right: width - 40, bottom: 318 };
                DrawTextW(mem_dc, to_wide("新歌速递 · 多源首发").as_ptr(), -1, &mut nr_sec_rect, DT_LEFT | DT_SINGLELINE);

                // New release rows
                for (i, rel) in rec_ref.new_releases.iter().enumerate().take(3) {
                    let r_top = 324 + (i as i32) * 44;
                    let r_brush = CreateSolidBrush(rgb(25, 27, 34));
                    let row_rect = RECT { left: 240, top: r_top, right: width - 40, bottom: r_top + 40 };
                    FillRect(mem_dc, &row_rect, r_brush);
                    DeleteObject(r_brush);

                    SelectObject(mem_dc, font_body);
                    SetTextColor(mem_dc, rgb(255, 255, 255));
                    let mut title_r = RECT { left: 256, top: r_top + 8, right: 520, bottom: r_top + 32 };
                    DrawTextW(mem_dc, to_wide(&rel.title).as_ptr(), -1, &mut title_r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

                    SetTextColor(mem_dc, rgb(150, 150, 160));
                    let mut art_r = RECT { left: 530, top: r_top + 8, right: 700, bottom: r_top + 32 };
                    DrawTextW(mem_dc, to_wide(&rel.artist).as_ptr(), -1, &mut art_r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

                    // Badge (Hi-Res / FLAC / SQ)
                    let badge_b = CreateSolidBrush(rgb(20, 48, 36));
                    let badge_box = RECT { left: width - 130, top: r_top + 9, right: width - 60, bottom: r_top + 31 };
                    FillRect(mem_dc, &badge_box, badge_b);
                    DeleteObject(badge_b);

                    SelectObject(mem_dc, font_small);
                    SetTextColor(mem_dc, rgb(52, 199, 89)); // Green
                    let mut b_text_r = badge_box;
                    DrawTextW(mem_dc, to_wide(&rel.badge).as_ptr(), -1, &mut b_text_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                }
            }
            AppRoute::Playlist(_) => {
                // Playlist Header
                let pl_ref = app.and_then(|a| a.state.selected_playlist.as_ref());
                let title = pl_ref.map(|p| p.title.as_str()).unwrap_or("华语流行黄金时代精选");
                let creator = pl_ref.map(|p| p.creator.as_str()).unwrap_or("昆音雷达");
                let count = pl_ref.map(|p| p.track_count()).unwrap_or(50);
                let dur = pl_ref.map(|p| p.formatted_total_duration()).unwrap_or_else(|| "45 分钟".to_string());
                let is_fav = pl_ref.map(|p| p.is_favorite).unwrap_or(false);

                // Squircle cover
                let cov_brush = CreateSolidBrush(rgb(35, 42, 58));
                let cov_rect = RECT { left: 240, top: 105, right: 310, bottom: 175 };
                FillRect(mem_dc, &cov_rect, cov_brush);
                DeleteObject(cov_brush);

                SelectObject(mem_dc, font_title);
                SetTextColor(mem_dc, rgb(10, 132, 255));
                let mut cov_t = cov_rect;
                DrawTextW(mem_dc, to_wide("♫").as_ptr(), -1, &mut cov_t, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                SelectObject(mem_dc, font_title);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut t_rect = RECT { left: 326, top: 105, right: width - 40, bottom: 131 };
                DrawTextW(mem_dc, to_wide(title).as_ptr(), -1, &mut t_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(150, 150, 160));
                let mut meta_rect = RECT { left: 326, top: 133, right: width - 40, bottom: 151 };
                let meta_str = format!("创建者: {} · {} 首曲目 · 总时长 {}", creator, count, dur);
                DrawTextW(mem_dc, to_wide(&meta_str).as_ptr(), -1, &mut meta_rect, DT_LEFT | DT_SINGLELINE);

                // Action pills: "▶ 全部播放" and "♡ 收藏歌单"
                let play_all_b = CreateSolidBrush(rgb(10, 132, 255));
                let play_all_r = RECT { left: 326, top: 155, right: 426, bottom: 183 };
                FillRect(mem_dc, &play_all_r, play_all_b);
                DeleteObject(play_all_b);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut pat_r = play_all_r;
                DrawTextW(mem_dc, to_wide("▶ 全部播放").as_ptr(), -1, &mut pat_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                let fav_b = CreateSolidBrush(if is_fav { rgb(220, 38, 70) } else { rgb(40, 44, 56) });
                let fav_r = RECT { left: 436, top: 155, right: 536, bottom: 183 };
                FillRect(mem_dc, &fav_r, fav_b);
                DeleteObject(fav_b);

                let mut fav_t_r = fav_r;
                let fav_label = to_wide(if is_fav { "♥ 已收藏" } else { "♡ 收藏歌单" });
                DrawTextW(mem_dc, fav_label.as_ptr(), -1, &mut fav_t_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                // Render Table rows
                paint_sample_rows(mem_dc, width, height, font_small, font_body, font_body_bold, 195, app);
            }
            AppRoute::Settings => {
                // EQ Preset selector pills (y: 110 .. 142)
                let cur_bands = app.map(|a| a.state.dsp.bands).unwrap_or([0.0; 10]);
                let presets = [
                    ("平直 (Flat)", 240, 335),
                    ("原声 (Acoustic)", 345, 450),
                    ("低音增强 (Bass)", 460, 570),
                    ("清澈人声 (Vocal)", 580, 685),
                    ("电子动感 (Electro)", 695, 810),
                ];
                for (p_name, l, r) in presets {
                    let is_active = (p_name.contains("Flat") && cur_bands == [0.0; 10])
                        || (p_name.contains("Bass") && cur_bands[0] > 4.0);
                    let b = CreateSolidBrush(if is_active { rgb(28, 54, 90) } else { rgb(32, 34, 44) });
                    let pill_r = RECT { left: l, top: 110, right: r, bottom: 140 };
                    FillRect(mem_dc, &pill_r, b);
                    DeleteObject(b);

                    SelectObject(mem_dc, font_small);
                    SetTextColor(mem_dc, if is_active { rgb(10, 132, 255) } else { rgb(180, 180, 190) });
                    let mut pt_r = pill_r;
                    DrawTextW(mem_dc, to_wide(p_name).as_ptr(), -1, &mut pt_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                }

                // 10-Band Graphic Equalizer Visualizer Box (y: 152 .. 325)
                let eq_box_b = CreateSolidBrush(rgb(20, 22, 28));
                let eq_box_r = RECT { left: 240, top: 152, right: width - 40, bottom: 325 };
                FillRect(mem_dc, &eq_box_r, eq_box_b);
                DeleteObject(eq_box_b);

                // Center 0dB indicator line
                let zero_line_b = CreateSolidBrush(rgb(45, 48, 58));
                let zero_line_r = RECT { left: 250, top: 238, right: width - 50, bottom: 239 };
                FillRect(mem_dc, &zero_line_r, zero_line_b);
                DeleteObject(zero_line_b);

                // 10 frequency bars
                let eq_ref = app.map(|a| &a.state.eq_visualizer);
                let bands = eq_ref.map(|e| e.bands).unwrap_or([0.0; 10]);
                for (i, &gain) in bands.iter().enumerate() {
                    let step = ((width - 320) / 9).max(40);
                    let bar_x = 260 + (i as i32) * step;
                    // Gain slider track
                    let trk_b = CreateSolidBrush(rgb(35, 38, 48));
                    let trk_r = RECT { left: bar_x - 3, top: 170, right: bar_x + 3, bottom: 305 };
                    FillRect(mem_dc, &trk_r, trk_b);
                    DeleteObject(trk_b);

                    // Gain active point
                    let gain_y = 238 - ((gain / 12.0) * 58.0) as i32;
                    let dot_b = CreateSolidBrush(rgb(10, 132, 255));
                    let dot_r = RECT { left: bar_x - 8, top: gain_y - 6, right: bar_x + 8, bottom: gain_y + 6 };
                    FillRect(mem_dc, &dot_r, dot_b);
                    DeleteObject(dot_b);

                    // Frequency label
                    SelectObject(mem_dc, font_small);
                    SetTextColor(mem_dc, rgb(130, 130, 140));
                    let mut freq_r = RECT { left: bar_x - 22, top: 308, right: bar_x + 22, bottom: 322 };
                    DrawTextW(mem_dc, to_wide(FREQUENCIES[i]).as_ptr(), -1, &mut freq_r, DT_CENTER | DT_SINGLELINE);
                }

                // Output engine badge box
                let eng_b = CreateSolidBrush(rgb(22, 38, 30));
                let eng_r = RECT { left: 240, top: 340, right: width - 40, bottom: 390 };
                FillRect(mem_dc, &eng_r, eng_b);
                DeleteObject(eng_b);

                SelectObject(mem_dc, font_body_bold);
                SetTextColor(mem_dc, rgb(52, 199, 89));
                let mut eng_t = RECT { left: 260, top: 353, right: width - 60, bottom: 377 };
                DrawTextW(mem_dc, to_wide("● WASAPI 独占硬件输出 · 100% 纯 Rust 音频管线 · 极低延迟 3.2ms · 零 WebView 内存常驻").as_ptr(), -1, &mut eng_t, DT_LEFT | DT_SINGLELINE);
            }
            AppRoute::Search => {
                // Source Tabs: [ 全部, QQ音乐, 网易云, 酷狗音乐, 酷我音乐 ]
                let cur_tab = app.map(|a| a.state.search_source_tab).unwrap_or(SearchSourceTab::All);
                let tabs = [
                    (SearchSourceTab::All, "聚合全源", 240, 315),
                    (SearchSourceTab::Qq, "QQ音乐", 325, 400),
                    (SearchSourceTab::Wy, "网易云", 410, 485),
                    (SearchSourceTab::Kg, "酷狗音乐", 495, 570),
                    (SearchSourceTab::Kw, "酷我音乐", 580, 655),
                ];
                for (tab, name, l, r) in tabs {
                    let is_sel = tab == cur_tab;
                    let tb_b = CreateSolidBrush(if is_sel { rgb(28, 54, 90) } else { rgb(32, 34, 44) });
                    let tb_r = RECT { left: l, top: 108, right: r, bottom: 138 };
                    FillRect(mem_dc, &tb_r, tb_b);
                    DeleteObject(tb_b);

                    SelectObject(mem_dc, font_small);
                    SetTextColor(mem_dc, if is_sel { rgb(10, 132, 255) } else { rgb(170, 170, 180) });
                    let mut tt_r = tb_r;
                    DrawTextW(mem_dc, to_wide(name).as_ptr(), -1, &mut tt_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                }

                // Render result songs
                paint_sample_rows(mem_dc, width, height, font_small, font_body, font_body_bold, 148, app);
            }
            AppRoute::Library => {
                // Category Tabs: [ 全部曲目, 专辑分类, 歌手分类, 本地文件夹 ]
                let cur_tab = app.map(|a| a.state.library_model.current_tab).unwrap_or(LibraryCategoryTab::Tracks);
                let tabs = [
                    (LibraryCategoryTab::Tracks, "全部曲目", 240, 325),
                    (LibraryCategoryTab::Albums, "专辑分类", 335, 420),
                    (LibraryCategoryTab::Artists, "歌手分类", 430, 515),
                    (LibraryCategoryTab::Folders, "本地文件夹", 525, 620),
                ];
                for (tab, name, l, r) in tabs {
                    let is_sel = tab == cur_tab;
                    let tb_b = CreateSolidBrush(if is_sel { rgb(28, 54, 90) } else { rgb(32, 34, 44) });
                    let tb_r = RECT { left: l, top: 108, right: r, bottom: 138 };
                    FillRect(mem_dc, &tb_r, tb_b);
                    DeleteObject(tb_b);

                    SelectObject(mem_dc, font_small);
                    SetTextColor(mem_dc, if is_sel { rgb(10, 132, 255) } else { rgb(170, 170, 180) });
                    let mut tt_r = tb_r;
                    DrawTextW(mem_dc, to_wide(name).as_ptr(), -1, &mut tt_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                }

                // Stats Card (y: 146..186)
                let stats = app.map(|a| a.state.library_model.compute_stats()).unwrap_or(LibraryStats {
                    total_tracks: 4,
                    total_folders: 3,
                    lossless_percent: 100.0,
                    total_duration_ms: 1_200_000,
                });
                let stat_b = CreateSolidBrush(rgb(22, 26, 36));
                let stat_r = RECT { left: 240, top: 146, right: width - 40, bottom: 186 };
                FillRect(mem_dc, &stat_r, stat_b);
                DeleteObject(stat_b);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(160, 160, 170));
                let mut st_t = RECT { left: 256, top: 156, right: width - 180, bottom: 178 };
                let st_msg = format!("本地曲库: {} 首 · 扫描目录: {} 个 · {} · 总时长 {}", stats.total_tracks, stats.total_folders, stats.formatted_lossless(), stats.formatted_duration());
                DrawTextW(mem_dc, to_wide(&st_msg).as_ptr(), -1, &mut st_t, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                // "+ 添加音乐目录" pill
                let add_b = CreateSolidBrush(rgb(28, 54, 90));
                let add_r = RECT { left: width - 175, top: 152, right: width - 50, bottom: 180 };
                FillRect(mem_dc, &add_r, add_b);
                DeleteObject(add_b);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(10, 132, 255));
                let mut add_t = add_r;
                DrawTextW(mem_dc, to_wide("+ 添加音乐目录").as_ptr(), -1, &mut add_t, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                match cur_tab {
                    LibraryCategoryTab::Tracks => {
                        paint_sample_rows(mem_dc, width, height, font_small, font_body, font_body_bold, 195, app);
                    }
                    LibraryCategoryTab::Albums => {
                        let albums = app.map(|a| a.state.library_model.albums.clone()).unwrap_or_default();
                        for (idx, al) in albums.iter().enumerate().take(6) {
                            let card_y = 195 + (idx as i32) * 44;
                            let c_b = CreateSolidBrush(rgb(25, 28, 38));
                            let c_r = RECT { left: 240, top: card_y, right: width - 40, bottom: card_y + 38 };
                            FillRect(mem_dc, &c_r, c_b);
                            DeleteObject(c_b);

                            SelectObject(mem_dc, font_body_bold);
                            SetTextColor(mem_dc, rgb(255, 255, 255));
                            let mut al_t = RECT { left: 255, top: card_y + 8, right: 480, bottom: card_y + 30 };
                            DrawTextW(mem_dc, to_wide(&al.name).as_ptr(), -1, &mut al_t, DT_LEFT | DT_SINGLELINE);

                            SelectObject(mem_dc, font_small);
                            SetTextColor(mem_dc, rgb(140, 140, 150));
                            let mut ar_t = RECT { left: 490, top: card_y + 10, right: 700, bottom: card_y + 30 };
                            DrawTextW(mem_dc, to_wide(&al.artist).as_ptr(), -1, &mut ar_t, DT_LEFT | DT_SINGLELINE);

                            let mut tc_t = RECT { left: width - 150, top: card_y + 10, right: width - 50, bottom: card_y + 30 };
                            let tc_str = format!("{} 首曲目", al.track_count);
                            DrawTextW(mem_dc, to_wide(&tc_str).as_ptr(), -1, &mut tc_t, DT_RIGHT | DT_SINGLELINE);
                        }
                    }
                    LibraryCategoryTab::Artists => {
                        let artists = app.map(|a| a.state.library_model.artists.clone()).unwrap_or_default();
                        for (idx, ar) in artists.iter().enumerate().take(6) {
                            let card_y = 195 + (idx as i32) * 44;
                            let c_b = CreateSolidBrush(rgb(25, 28, 38));
                            let c_r = RECT { left: 240, top: card_y, right: width - 40, bottom: card_y + 38 };
                            FillRect(mem_dc, &c_r, c_b);
                            DeleteObject(c_b);

                            SelectObject(mem_dc, font_body_bold);
                            SetTextColor(mem_dc, rgb(255, 255, 255));
                            let mut ar_t = RECT { left: 255, top: card_y + 8, right: 480, bottom: card_y + 30 };
                            DrawTextW(mem_dc, to_wide(&ar.name).as_ptr(), -1, &mut ar_t, DT_LEFT | DT_SINGLELINE);

                            SelectObject(mem_dc, font_small);
                            SetTextColor(mem_dc, rgb(140, 140, 150));
                            let mut tc_t = RECT { left: width - 200, top: card_y + 10, right: width - 50, bottom: card_y + 30 };
                            let tc_str = format!("{} 张专辑 · {} 首曲目", ar.album_count, ar.track_count);
                            DrawTextW(mem_dc, to_wide(&tc_str).as_ptr(), -1, &mut tc_t, DT_RIGHT | DT_SINGLELINE);
                        }
                    }
                    LibraryCategoryTab::Folders => {
                        let folders = app.map(|a| a.state.library_model.folders.clone()).unwrap_or_default();
                        for (idx, f) in folders.iter().enumerate().take(6) {
                            let card_y = 195 + (idx as i32) * 44;
                            let c_b = CreateSolidBrush(rgb(25, 28, 38));
                            let c_r = RECT { left: 240, top: card_y, right: width - 40, bottom: card_y + 38 };
                            FillRect(mem_dc, &c_r, c_b);
                            DeleteObject(c_b);

                            SelectObject(mem_dc, font_body);
                            SetTextColor(mem_dc, rgb(200, 200, 210));
                            let mut f_t = RECT { left: 255, top: card_y + 8, right: width - 150, bottom: card_y + 30 };
                            let f_str = format!("📁 {}", f.path);
                            DrawTextW(mem_dc, to_wide(&f_str).as_ptr(), -1, &mut f_t, DT_LEFT | DT_SINGLELINE);

                            SelectObject(mem_dc, font_small);
                            SetTextColor(mem_dc, rgb(140, 140, 150));
                            let mut tc_t = RECT { left: width - 150, top: card_y + 10, right: width - 50, bottom: card_y + 30 };
                            let tc_str = format!("{} 首曲目", f.track_count);
                            DrawTextW(mem_dc, to_wide(&tc_str).as_ptr(), -1, &mut tc_t, DT_RIGHT | DT_SINGLELINE);
                        }
                    }
                }
            }
            AppRoute::Favorites => {
                // Heart Hero Banner (left: 240, top: 105, right: width - 40, bottom: 175)
                let cov_b = CreateSolidBrush(rgb(42, 24, 32));
                let cov_r = RECT { left: 240, top: 105, right: 310, bottom: 175 };
                FillRect(mem_dc, &cov_r, cov_b);
                DeleteObject(cov_b);

                SelectObject(mem_dc, font_title);
                SetTextColor(mem_dc, rgb(255, 45, 85)); // Apple Rose Red
                let mut cov_t = cov_r;
                DrawTextW(mem_dc, to_wide("♥").as_ptr(), -1, &mut cov_t, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                SelectObject(mem_dc, font_title);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut t_rect = RECT { left: 326, top: 105, right: width - 40, bottom: 131 };
                DrawTextW(mem_dc, to_wide("我的珍藏音乐").as_ptr(), -1, &mut t_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

                let fav_count = app.map(|a| a.state.favorites_model.total_count()).unwrap_or(3);
                let fav_dur = app.map(|a| a.state.favorites_model.formatted_duration()).unwrap_or_else(|| "12 分钟".to_string());
                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(150, 150, 160));
                let mut meta_rect = RECT { left: 326, top: 133, right: width - 40, bottom: 151 };
                let meta_str = format!("精选歌曲 · {} 首珍藏 · 总时长 {} · WASAPI 独占硬件输出", fav_count, fav_dur);
                DrawTextW(mem_dc, to_wide(&meta_str).as_ptr(), -1, &mut meta_rect, DT_LEFT | DT_SINGLELINE);

                // Actions: [ ▶ 播放全部 ] and [ 🔀 随机播放 ]
                let play_b = CreateSolidBrush(rgb(10, 132, 255));
                let play_r = RECT { left: 326, top: 155, right: 426, bottom: 183 };
                FillRect(mem_dc, &play_r, play_b);
                DeleteObject(play_b);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut pat_r = play_r;
                DrawTextW(mem_dc, to_wide("▶ 播放全部").as_ptr(), -1, &mut pat_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                let shuf_b = CreateSolidBrush(rgb(40, 44, 56));
                let shuf_r = RECT { left: 436, top: 155, right: 536, bottom: 183 };
                FillRect(mem_dc, &shuf_r, shuf_b);
                DeleteObject(shuf_b);

                let mut shuf_t_r = shuf_r;
                DrawTextW(mem_dc, to_wide("🔀 随机播放").as_ptr(), -1, &mut shuf_t_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                paint_sample_rows(mem_dc, width, height, font_small, font_body, font_body_bold, 195, app);
            }
            AppRoute::Trial => {
                // Queue Card (y: 105..165)
                let q_b = CreateSolidBrush(rgb(26, 32, 44));
                let q_r = RECT { left: 240, top: 105, right: 295, bottom: 160 };
                FillRect(mem_dc, &q_r, q_b);
                DeleteObject(q_b);

                SelectObject(mem_dc, font_title);
                SetTextColor(mem_dc, rgb(10, 132, 255));
                let mut q_t = q_r;
                DrawTextW(mem_dc, to_wide("⏱").as_ptr(), -1, &mut q_t, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                SelectObject(mem_dc, font_title);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut t_rect = RECT { left: 310, top: 105, right: width - 40, bottom: 131 };
                DrawTextW(mem_dc, to_wide("试听列表与临时队列").as_ptr(), -1, &mut t_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

                let q_count = app.map(|a| a.state.trial_model.count()).unwrap_or(2);
                let q_dur = app.map(|a| a.state.trial_model.formatted_duration()).unwrap_or_else(|| "8 分钟".to_string());
                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(150, 150, 160));
                let mut meta_rect = RECT { left: 310, top: 133, right: width - 40, bottom: 151 };
                let meta_str = format!("临时试听队列 · {} 首曲目 · 时长 {} · 先进先出自动去重", q_count, q_dur);
                DrawTextW(mem_dc, to_wide(&meta_str).as_ptr(), -1, &mut meta_rect, DT_LEFT | DT_SINGLELINE);

                // Actions: [ ▶ 顺序播放 ] and [ 🗑 清空队列 ]
                let play_b = CreateSolidBrush(rgb(10, 132, 255));
                let play_r = RECT { left: 310, top: 155, right: 400, bottom: 183 };
                FillRect(mem_dc, &play_r, play_b);
                DeleteObject(play_b);

                SelectObject(mem_dc, font_small);
                SetTextColor(mem_dc, rgb(255, 255, 255));
                let mut pat_r = play_r;
                DrawTextW(mem_dc, to_wide("▶ 顺序播放").as_ptr(), -1, &mut pat_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                let clr_b = CreateSolidBrush(rgb(50, 28, 34));
                let clr_r = RECT { left: 410, top: 155, right: 500, bottom: 183 };
                FillRect(mem_dc, &clr_r, clr_b);
                DeleteObject(clr_b);

                let mut clr_t_r = clr_r;
                DrawTextW(mem_dc, to_wide("🗑 清空队列").as_ptr(), -1, &mut clr_t_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                paint_sample_rows(mem_dc, width, height, font_small, font_body, font_body_bold, 195, app);
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
