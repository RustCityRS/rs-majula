#[cfg(rev = "225")]
pub const FRAME: &[(&str, usize, usize, usize, usize)] = &[
    ("backleft1", 0, 11, 8, 334),
    ("backleft2", 0, 375, 22, 96),
    ("backright1", 729, 5, 60, 166),
    ("backright2", 752, 231, 37, 261),
    ("backtop1", 0, 0, 561, 11),
    ("backtop2", 561, 0, 228, 5),
    ("backvmid1", 520, 11, 41, 154),
    ("backvmid2", 520, 231, 42, 114),
    ("backvmid3", 501, 375, 61, 117),
    ("backhmid2", 0, 345, 562, 30),
    ("backhmid1", 520, 165, 269, 66), // sub-buffer Rt -> screen (520,165)
    ("backbase1", 0, 471, 501, 61),   // sub-buffer Pt -> screen (0,471)
    ("backbase2", 501, 492, 288, 40), // sub-buffer Qt -> screen (501,492)
];
#[cfg(rev = "225")]
pub const CANVAS_W: usize = 789;
#[cfg(rev = "225")]
pub const CANVAS_H: usize = 532;

#[cfg(since_244)]
pub const FRAME: &[(&str, usize, usize, usize, usize)] = &[
    ("backtop1", 0, 0, 765, 4),
    ("backleft1", 0, 4, 4, 334),
    ("backright1", 722, 4, 43, 156),
    ("backvmid1", 516, 4, 34, 156),
    ("backhmid1", 516, 160, 249, 45),
    ("backvmid2", 516, 205, 37, 133),
    ("backright2", 743, 205, 22, 261),
    ("backhmid2", 0, 338, 553, 19),
    ("backleft2", 0, 357, 17, 96),
    ("backvmid3", 496, 357, 57, 109),
    ("backbase1", 0, 453, 496, 50),
    ("backbase2", 496, 466, 269, 37),
];
#[cfg(since_244)]
pub const CANVAS_W: usize = 765;
#[cfg(since_244)]
pub const CANVAS_H: usize = 503;
