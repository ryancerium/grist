use nom::character::complete::space0;
use nom::sequence::delimited;
use nom::character::complete::alphanumeric1;
use nom::IResult;

/*
MonXLeft
MonXRight
MonXTop
MonXBottom
MonXWidth
MonXHeight

MonLeft
MonRight
MonTop
MonBottom
MonWidth
MonHeight

LeftCurrent
    Keys LeftWindow Left
    Area MonLeft MonTop (MonWidth / 2) MonHeight

RightCurrent
    Keys LeftWindow Right
    Area (MonLeft + MonWidth / 2) MonTop (MonWidth / 2) MonHeight

North
    Keys LeftWindow Numpad8
    Action North

Maximize
    Keys LeftWindow Up
    Action Maximize
*/

fn name(i: &str) -> IResult<&str, &str> {
    delimited(space0, alphanumeric1, space0)(i)
}

enum Values {4
    Keys(Vec<hotkey_action::VK>),
    Area(RECT),
    Action(hotkey_action::Action),
}

enum Actions {
    Area(String),
    North,
    South,
    East,
    West,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Maximize,
    Minimize,
}

pub fn parse_area(i: &str) -> IResult<&str, (i32, i32, i32, i32)> {
    tuple(expr, expr, expr, expr)(i)?
    //Ok((0, 0, 0, 0))
}

pub fn parse_actions(i: &str) {
    let (i, name) = name(i)?
    let (i, keys) = keys(i)?
    let (i, doing) = action(i)?
    match doing {
        Area(s) => parse_area(),
        North => north,
        Maximize => maximize(),
    }
}

#[test]
pub fn test_named_field {

}
