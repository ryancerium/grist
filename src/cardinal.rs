use std::cmp::{max, min};
use winapi::shared::windef::{POINT, RECT};

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
        return POINT {
            x: self.left,
            y: self.top,
        };
    }

    fn top_right(&self) -> POINT {
        return POINT {
            x: self.right,
            y: self.top,
        };
    }

    fn bottom_left(&self) -> POINT {
        return POINT {
            x: self.left,
            y: self.bottom,
        };
    }

    fn bottom_right(&self) -> POINT {
        return POINT {
            x: self.right,
            y: self.bottom,
        };
    }

    fn center(&self) -> POINT {
        return POINT {
            x: (self.left + self.right) / 2,
            y: (self.top + self.bottom) / 2,
        };
    }
}

pub fn make_rect(p0: &POINT, p1: &POINT) -> RECT {
    return RECT {
        left: min(p0.x, p1.x),
        right: max(p0.x, p1.x),
        top: min(p0.y, p1.y),
        bottom: max(p0.y, p1.y),
    };
}

pub fn default_rect() -> RECT {
    RECT {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    }
}
