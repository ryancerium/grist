use std::cmp::{max, min};
use windows::Win32::Foundation::{POINT, RECT};

pub trait Cardinal {
    fn width(&self) -> i32;
    fn height(&self) -> i32;

    fn north(&self) -> POINT;
    fn south(&self) -> POINT;
    fn east(&self) -> POINT;
    fn west(&self) -> POINT;

    fn top_left(&self) -> POINT;
    fn top_right(&self) -> POINT;
    fn bottom_left(&self) -> POINT;
    fn bottom_right(&self) -> POINT;

    fn center(&self) -> POINT;

    fn from_points(p0: POINT, p1: POINT) -> Self;
}

impl Cardinal for RECT {
    fn width(&self) -> i32 {
        self.right - self.left
    }

    fn height(&self) -> i32 {
        self.bottom - self.top
    }

    fn north(&self) -> POINT {
        POINT {
            x: self.center().x,
            y: self.top,
        }
    }

    fn south(&self) -> POINT {
        POINT {
            x: self.center().x,
            y: self.bottom,
        }
    }

    fn east(&self) -> POINT {
        POINT {
            x: self.right,
            y: self.center().y,
        }
    }

    fn west(&self) -> POINT {
        POINT {
            x: self.left,
            y: self.center().y,
        }
    }

    fn top_left(&self) -> POINT {
        POINT { x: self.left, y: self.top }
    }

    fn top_right(&self) -> POINT {
        POINT { x: self.right, y: self.top }
    }

    fn bottom_left(&self) -> POINT {
        POINT { x: self.left, y: self.bottom }
    }

    fn bottom_right(&self) -> POINT {
        POINT {
            x: self.right,
            y: self.bottom,
        }
    }

    fn center(&self) -> POINT {
        POINT {
            x: (self.left + self.right) / 2,
            y: (self.top + self.bottom) / 2,
        }
    }

    fn from_points(p0: POINT, p1: POINT) -> RECT {
        RECT {
            left: min(p0.x, p1.x),
            right: max(p0.x, p1.x),
            top: min(p0.y, p1.y),
            bottom: max(p0.y, p1.y),
        }
    }
}
