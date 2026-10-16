use crate::image::{Image, Rect};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    Select,
    Move,
    Pen,
    Line,
    Arrow,
    Rectangle,
    Ellipse,
    Highlight,
    Text,
    Blur,
    Pixelate,
    Redact,
    Step,
    Picker,
}
impl Tool {
    pub fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Select => "Область",
            Self::Move => "Перемещение",
            Self::Pen => "Карандаш",
            Self::Line => "Линия",
            Self::Arrow => "Стрелка",
            Self::Rectangle => "Прямоугольник",
            Self::Ellipse => "Овал",
            Self::Highlight => "Маркер",
            Self::Text => "Текст",
            Self::Blur => "Blur",
            Self::Pixelate => "Pixelate",
            Self::Redact => "Скрыть данные",
            Self::Step => "Номер шага",
            Self::Picker => "Пипетка",
        })
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Select => "V",
            Self::Move => "M",
            Self::Pen => "P",
            Self::Line => "L",
            Self::Arrow => "A",
            Self::Rectangle => "R",
            Self::Ellipse => "O",
            Self::Highlight => "H",
            Self::Text => "T",
            Self::Blur => "B",
            Self::Pixelate => "X",
            Self::Redact => "D",
            Self::Step => "N",
            Self::Picker => "I",
        }
    }
}
pub const TOOLS: [Tool; 11] = [
    Tool::Pen,
    Tool::Line,
    Tool::Arrow,
    Tool::Rectangle,
    Tool::Ellipse,
    Tool::Highlight,
    Tool::Text,
    Tool::Redact,
    Tool::Step,
    Tool::Picker,
    Tool::Move,
];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub tool: Tool,
    pub points: Vec<Point>,
    pub text: String,
    pub color: [u8; 4],
    pub width: f32,
    pub number: u32,
}
impl Annotation {
    pub fn new(tool: Tool, p: Point, color: [u8; 4], width: f32) -> Self {
        Self {
            tool,
            points: vec![p, p],
            text: String::new(),
            color,
            width,
            number: 1,
        }
    }
    pub fn rect(&self) -> Rect {
        let mut minx = f32::MAX;
        let mut miny = f32::MAX;
        let mut maxx = f32::MIN;
        let mut maxy = f32::MIN;
        for p in &self.points {
            minx = minx.min(p.x);
            miny = miny.min(p.y);
            maxx = maxx.max(p.x);
            maxy = maxy.max(p.y)
        }
        if self.tool == Tool::Text {
            maxx = maxx.max(minx + self.text.chars().count() as f32 * self.width * 5.5);
            maxy = maxy.max(miny + self.width * 10.0)
        }
        if self.tool == Tool::Step {
            minx -= 18.;
            maxx += 18.;
            miny -= 18.;
            maxy += 18.;
        }
        Rect {
            x: minx.floor() as i32,
            y: miny.floor() as i32,
            w: (maxx - minx).ceil().max(1.) as u32,
            h: (maxy - miny).ceil().max(1.) as u32,
        }
    }
    pub fn translate(&mut self, dx: f32, dy: f32) {
        for p in &mut self.points {
            p.x += dx;
            p.y += dy
        }
    }
    pub fn hit(&self, p: Point) -> bool {
        let r = self.rect();
        Rect {
            x: r.x - 8,
            y: r.y - 8,
            w: r.w + 16,
            h: r.h + 16,
        }
        .contains(p.x as i32, p.y as i32)
    }
}
#[derive(Clone)]
enum Change {
    Add {
        index: usize,
        shape: Annotation,
    },
    Remove {
        index: usize,
        shape: Annotation,
    },
    Replace {
        index: usize,
        before: Annotation,
        after: Annotation,
    },
}
pub struct Editor {
    pub image: Image,
    pub selection: Option<Rect>,
    pub annotations: Vec<Annotation>,
    pub tool: Tool,
    pub color: [u8; 4],
    pub tool_colors: std::collections::BTreeMap<String, [u8; 4]>,
    pub width: f32,
    pub selected: Option<usize>,
    pub draft: Option<Annotation>,
    undo: Vec<Change>,
    redo: Vec<Change>,
}
impl Editor {
    pub fn new(image: Image) -> Self {
        Self {
            image,
            selection: None,
            annotations: vec![],
            tool: Tool::Pen,
            color: [39, 131, 222, 255],
            tool_colors: Default::default(),
            width: 3.,
            selected: None,
            draft: None,
            undo: vec![],
            redo: vec![],
        }
    }
    pub fn bounds(&self) -> Rect {
        Rect {
            x: 0,
            y: 0,
            w: self.image.width,
            h: self.image.height,
        }
    }
    pub fn selection(&self) -> Option<Rect> {
        self.selection
            .map(|r| r.clamp(self.image.width, self.image.height))
            .filter(|r| r.valid())
    }
    pub fn add(&mut self, shape: Annotation) {
        let index = self.annotations.len();
        self.annotations.push(shape.clone());
        self.undo.push(Change::Add { index, shape });
        self.redo.clear();
        self.selected = Some(index)
    }
    pub fn delete_selected(&mut self) {
        if let Some(i) = self.selected.take() {
            if i < self.annotations.len() {
                let shape = self.annotations.remove(i);
                self.undo.push(Change::Remove { index: i, shape });
                self.redo.clear();
            }
        }
    }
    pub fn replace(&mut self, index: usize, before: Annotation, after: Annotation) {
        if before != after {
            self.annotations[index] = after.clone();
            self.undo.push(Change::Replace {
                index,
                before,
                after,
            });
            self.redo.clear()
        }
    }
    pub fn undo(&mut self) {
        if let Some(c) = self.undo.pop() {
            match &c {
                Change::Add { index, .. } => {
                    self.annotations.remove(*index);
                }
                Change::Remove { index, shape } => self.annotations.insert(*index, shape.clone()),
                Change::Replace { index, before, .. } => self.annotations[*index] = before.clone(),
            };
            self.redo.push(c);
            self.selected = None
        }
    }
    pub fn redo(&mut self) {
        if let Some(c) = self.redo.pop() {
            match &c {
                Change::Add { index, shape } => self.annotations.insert(*index, shape.clone()),
                Change::Remove { index, .. } => {
                    self.annotations.remove(*index);
                }
                Change::Replace { index, after, .. } => self.annotations[*index] = after.clone(),
            };
            self.undo.push(c);
            self.selected = None
        }
    }
    pub fn pick(&mut self, p: Point) {
        self.selected = self.annotations.iter().rposition(|a| a.hit(p));
    }
    pub fn next_number(&self) -> u32 {
        self.annotations
            .iter()
            .filter(|a| a.tool == Tool::Step)
            .map(|a| a.number)
            .max()
            .unwrap_or(0)
            + 1
    }
}

/// Hit-test selection borders in image coordinates; 8 directions, independent of active tool.
pub fn selection_edge(r: Rect, p: Point, tolerance: f32) -> Option<u8> {
    let (x, y) = (p.x, p.y);
    let (l, t, rr, b) = (
        r.x as f32,
        r.y as f32,
        (r.x + r.w as i32) as f32,
        (r.y + r.h as i32) as f32,
    );
    if x < l - tolerance || x > rr + tolerance || y < t - tolerance || y > b + tolerance {
        return None;
    }
    let mut edge = 0;
    if (x - l).abs() <= tolerance {
        edge |= 1
    } else if (x - rr).abs() <= tolerance {
        edge |= 2
    }
    if (y - t).abs() <= tolerance {
        edge |= 4
    } else if (y - b).abs() <= tolerance {
        edge |= 8
    }
    if edge == 0 {
        None
    } else {
        Some(edge)
    }
}
pub fn resize_selection(r: Rect, edge: u8, p: Point, w: u32, h: u32) -> Rect {
    let (mut l, mut t, mut rr, mut b) = (r.x, r.y, r.x + r.w as i32, r.y + r.h as i32);
    let x = (p.x.round() as i32).clamp(0, w as i32);
    let y = (p.y.round() as i32).clamp(0, h as i32);
    if edge & 1 != 0 {
        l = x.min(rr - 1)
    }
    if edge & 2 != 0 {
        rr = x.max(l + 1)
    }
    if edge & 4 != 0 {
        t = y.min(b - 1)
    }
    if edge & 8 != 0 {
        b = y.max(t + 1)
    }
    Rect {
        x: l,
        y: t,
        w: (rr - l) as u32,
        h: (b - t) as u32,
    }
    .clamp(w, h)
}
