#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod solver;

use eframe::egui::{self, Color32, RichText, Stroke, vec2};
use solver::{CELLS, SHAPES, SIZE};

const SHAPE_COLORS: [Color32; 4] = [
    Color32::from_rgb(70, 130, 220),
    Color32::from_rgb(60, 170, 110),
    Color32::from_rgb(170, 90, 200),
    Color32::from_rgb(200, 160, 40),
];

fn shape_name(shape: usize) -> String {
    let (h, w) = SHAPES[shape];
    format!("{h}×{w}")
}

#[derive(Clone)]
struct State {
    misses: u64,
    found: Vec<(usize, u64)>,
    remaining: [usize; 4],
}

impl State {
    fn blocked(&self) -> u64 {
        self.found.iter().fold(self.misses, |b, (_, m)| b | m)
    }

    // 存檔格式：一行一個狀態，「misses r0 r1 r2 r3 shape:mask ...」
    fn to_line(&self) -> String {
        let mut parts = vec![self.misses.to_string()];
        parts.extend(self.remaining.iter().map(|n| n.to_string()));
        parts.extend(self.found.iter().map(|(s, m)| format!("{s}:{m}")));
        parts.join(" ")
    }

    fn parse(line: &str) -> Option<State> {
        let mut it = line.split_whitespace();
        let misses = it.next()?.parse().ok()?;
        let mut remaining = [0; 4];
        for r in &mut remaining {
            *r = it.next()?.parse().ok()?;
        }
        let found = it
            .map(|t| {
                let (s, m) = t.split_once(':')?;
                let s: usize = s.parse().ok()?;
                (s < SHAPES.len()).then_some((s, m.parse().ok()?))
            })
            .collect::<Option<_>>()?;
        Some(State { misses, found, remaining })
    }
}

fn save_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| Some(p.parent()?.join("save.txt")))
        .unwrap_or_else(|| "save.txt".into())
}

struct Placing {
    shape: usize,
    candidates: Vec<u64>,
    index: usize,
}

#[derive(Default)]
struct App {
    inputs: [String; 4],
    setup_error: Option<String>,
    // 空 = 開始畫面；最後一筆是目前狀態
    history: Vec<State>,
    probs: Option<[f64; CELLS]>,
    selected: Option<usize>,
    placing: Option<Placing>,
    message: Option<String>,
}

impl App {
    fn state(&self) -> &State {
        self.history.last().unwrap()
    }

    fn all_found(&self) -> bool {
        self.state().remaining.iter().all(|&n| n == 0)
    }

    // 剩餘格子機率全為 0 或 1
    fn inferred(&self) -> bool {
        let blocked = self.state().blocked();
        self.probs.is_some_and(|p| {
            (0..CELLS).all(|i| blocked & (1 << i) != 0 || p[i] == 0.0 || p[i] == 1.0)
        })
    }

    fn finished(&self) -> bool {
        self.all_found() || self.inferred()
    }

    fn recompute(&mut self) {
        self.selected = None;
        self.placing = None;
        self.message = None;
        let s = self.state();
        self.probs = if self.all_found() {
            None
        } else {
            solver::probabilities(&s.remaining, s.blocked())
        };
    }

    fn push(&mut self, state: State) {
        self.history.push(state);
        self.recompute();
    }

    fn place(&mut self, shape: usize, mask: u64) {
        let mut s = self.state().clone();
        s.found.push((shape, mask));
        s.remaining[shape] -= 1;
        self.push(s);
    }

    fn start(&mut self) {
        let mut counts = [0usize; 4];
        for (count, input) in counts.iter_mut().zip(&self.inputs) {
            match input.trim().parse() {
                Ok(n) => *count = n,
                Err(_) => {
                    self.setup_error = Some("請為每種寶藏輸入數量（非負整數）".into());
                    return;
                }
            }
        }
        if counts.iter().all(|&n| n == 0) {
            self.setup_error = Some("至少需要一個寶藏".into());
            return;
        }
        let area: usize = counts.iter().zip(SHAPES).map(|(n, (h, w))| n * h * w).sum();
        if area > CELLS {
            self.setup_error = Some(format!("寶藏總面積 {area} 超過棋盤 {CELLS} 格"));
            return;
        }
        self.setup_error = None;
        self.history.clear();
        self.push(State { misses: 0, found: Vec::new(), remaining: counts });
        // push 已算過機率，None 代表放不下，退回開始畫面
        if self.probs.is_none() {
            self.history.clear();
            self.setup_error = Some("找不到合法擺法（放不下或搜尋超過上限）".into());
        }
    }

    fn save(&mut self) {
        let text: Vec<String> = self.history.iter().map(State::to_line).collect();
        self.message = Some(match std::fs::write(save_path(), text.join("\n")) {
            Ok(()) => "已存檔".into(),
            Err(e) => format!("存檔失敗：{e}"),
        });
    }

    fn load(&mut self) {
        let history: Option<Vec<State>> = std::fs::read_to_string(save_path())
            .ok()
            .and_then(|text| text.lines().map(State::parse).collect());
        match history {
            Some(h) if !h.is_empty() => {
                self.setup_error = None;
                self.history = h;
                self.recompute();
            }
            _ => self.setup_error = Some("找不到存檔或存檔損壞".into()),
        }
    }

    fn setup_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("輸入各寶藏數量");
        ui.add_space(8.0);
        egui::Grid::new("counts").spacing(vec2(12.0, 8.0)).show(ui, |ui| {
            for (shape, input) in self.inputs.iter_mut().enumerate() {
                ui.label(RichText::new(shape_name(shape)).color(SHAPE_COLORS[shape]).strong());
                ui.add(egui::TextEdit::singleline(input).desired_width(60.0));
                ui.end_row();
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("開始").clicked() {
                self.start();
            }
            if ui.add_enabled(save_path().exists(), egui::Button::new("載入存檔")).clicked() {
                self.load();
            }
        });
        if let Some(err) = &self.setup_error {
            ui.colored_label(Color32::LIGHT_RED, err);
        }
    }

    fn board_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.state();
        let blocked = s.blocked();
        let finished = self.finished();
        let best = match self.probs {
            Some(p) if !finished => solver::best_cell(&p, blocked),
            _ => None,
        };
        let preview = self.placing.as_ref().map(|p| (p.shape, p.candidates[p.index]));
        let mut clicked = None;

        egui::Grid::new("board").spacing(vec2(3.0, 3.0)).show(ui, |ui| {
            for row in 0..SIZE {
                for col in 0..SIZE {
                    let i = row * SIZE + col;
                    let bit = 1u64 << i;
                    let found = s.found.iter().find(|(_, m)| m & bit != 0);
                    let prob = self.probs.map(|p| p[i]);

                    let (text, fill) = if let Some((shape, _)) = preview.filter(|(_, m)| m & bit != 0) {
                        (shape_name(shape), Color32::from_rgb(230, 220, 90))
                    } else if let Some(&(shape, _)) = found {
                        (shape_name(shape), SHAPE_COLORS[shape])
                    } else if s.misses & bit != 0 {
                        (String::new(), Color32::from_gray(35))
                    } else {
                        (String::new(), Color32::from_gray(80))
                    };

                    let stroke = if self.selected == Some(i) {
                        Stroke::new(3.0, Color32::WHITE)
                    } else if best == Some(i) {
                        Stroke::new(3.0, Color32::RED)
                    } else if finished && blocked & bit == 0 && prob == Some(1.0) {
                        Stroke::new(3.0, Color32::GREEN)
                    } else {
                        Stroke::NONE
                    };

                    let button = egui::Button::new(RichText::new(text).color(Color32::WHITE))
                        .fill(fill)
                        .stroke(stroke)
                        .min_size(vec2(56.0, 56.0));
                    if ui.add(button).clicked() && blocked & bit == 0 {
                        clicked = Some(i);
                    }
                }
                ui.end_row();
            }
        });

        if let Some(i) = clicked
            && !self.all_found()
            && self.placing.is_none()
        {
            self.selected = Some(i);
            self.message = None;
        }
    }

    fn side_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.state().clone();

        ui.heading("剩餘寶藏");
        for (shape, n) in s.remaining.iter().enumerate() {
            ui.label(RichText::new(format!("{}：{n}", shape_name(shape))).color(SHAPE_COLORS[shape]));
        }
        ui.separator();

        if self.all_found() {
            ui.colored_label(Color32::GREEN, "所有寶藏都找到了");
        } else if self.probs.is_none() {
            ui.colored_label(Color32::LIGHT_RED, "找不到合法盤面（輸入矛盾或搜尋超過上限），請檢查後按上一步");
        } else if self.inferred() {
            ui.colored_label(Color32::GREEN, "已推斷出所有寶藏位置（綠框），仍可繼續輸入");
        } else if let Some(i) = solver::best_cell(&self.probs.unwrap(), s.blocked()) {
            ui.label(format!("推薦：第 {} 列 第 {} 行（紅框）", i / SIZE + 1, i % SIZE + 1));
        }
        ui.separator();

        if let Some(placing) = &mut self.placing {
            ui.label(format!(
                "{} 擺法 {}/{}",
                shape_name(placing.shape),
                placing.index + 1,
                placing.candidates.len()
            ));
            let n = placing.candidates.len();
            let mut confirm = false;
            let mut cancel = false;
            ui.horizontal(|ui| {
                if ui.button("◀").clicked() {
                    placing.index = (placing.index + n - 1) % n;
                }
                if ui.button("▶").clicked() {
                    placing.index = (placing.index + 1) % n;
                }
                confirm = ui.button("確認").clicked();
                cancel = ui.button("取消").clicked();
            });
            if confirm {
                let (shape, mask) = (placing.shape, placing.candidates[placing.index]);
                self.place(shape, mask);
            } else if cancel {
                self.placing = None;
            }
        } else if let Some(cell) = self.selected {
            ui.label(format!("選取：第 {} 列 第 {} 行", cell / SIZE + 1, cell % SIZE + 1));
            if ui.button("空格").clicked() {
                let mut next = s.clone();
                next.misses |= 1 << cell;
                self.push(next);
                return;
            }
            ui.horizontal_wrapped(|ui| {
                for shape in 0..SHAPES.len() {
                    if s.remaining[shape] == 0 {
                        continue;
                    }
                    if ui.button(RichText::new(shape_name(shape)).color(SHAPE_COLORS[shape])).clicked() {
                        let blocked = s.blocked();
                        let candidates: Vec<u64> = solver::positions(shape)
                            .into_iter()
                            .filter(|m| m & (1 << cell) != 0 && m & blocked == 0)
                            .collect();
                        match candidates.len() {
                            0 => self.message = Some("這格放不下此寶藏".into()),
                            1 => self.place(shape, candidates[0]),
                            _ => self.placing = Some(Placing { shape, candidates, index: 0 }),
                        }
                    }
                }
            });
        } else if !self.all_found() {
            ui.label("點棋盤上的格子來輸入結果");
        }

        if let Some(msg) = &self.message {
            ui.colored_label(Color32::LIGHT_RED, msg);
        }
        ui.separator();

        ui.horizontal(|ui| {
            if ui.add_enabled(self.history.len() > 1, egui::Button::new("上一步")).clicked() {
                self.history.pop();
                self.recompute();
            }
            if ui.button("存檔").clicked() {
                self.save();
            }
            if ui.button("停止").clicked() {
                self.history.clear();
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.history.is_empty() {
            egui::CentralPanel::default_margins().show(ui, |ui| self.setup_ui(ui));
            return;
        }
        egui::Panel::right("side").default_size(240.0).show(ui, |ui| self.side_ui(ui));
        if self.history.is_empty() {
            return;
        }
        egui::CentralPanel::default_margins().show(ui, |ui| self.board_ui(ui));
    }
}

fn setup_fonts(ctx: &egui::Context) {
    // egui 內建字型沒有中文，借用系統的微軟正黑體
    let Ok(bytes) = std::fs::read("C:/Windows/Fonts/msjh.ttc") else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("msjh".into(), egui::FontData::from_owned(bytes).into());
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("msjh".into());
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
#[test]
fn save_line_roundtrip() {
    let s = State { misses: 5, found: vec![(3, 7), (0, u64::MAX)], remaining: [1, 0, 2, 3] };
    let t = State::parse(&s.to_line()).unwrap();
    assert_eq!((t.misses, t.found, t.remaining), (s.misses, s.found, s.remaining));
    assert!(State::parse("1 2 3").is_none());
    assert!(State::parse("0 0 0 0 0 9:1").is_none());
}

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default().with_inner_size([820.0, 560.0]);
    #[cfg(has_app_icon)]
    let viewport = viewport.with_icon(
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
            .expect("embedded application icon must be a valid PNG"),
    );
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "寶藏小助手",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::<App>::default())
        }),
    )
}
