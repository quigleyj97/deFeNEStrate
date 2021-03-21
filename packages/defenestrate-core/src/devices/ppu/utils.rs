#[macro_export]
macro_rules! state {
    (get $reg: ident, $mb: expr) => {{
        $mb.ppu().state.$reg
    }};

    (set $reg: ident, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg = $val
    }};

    (add $reg: ident, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg += $val
    }};

    (sub $reg: ident, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg -= $val
    }};

    (and $reg: ident, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg &= $val
    }};

    (or $reg: ident, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg |= $val
    }};

    (xor $reg: ident, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg ^= $val
    }};

    (shl $reg: ident, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg <<= $val
    }};

    (set_arr $reg:ident, $idx: expr, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg[$idx as usize] = $val
    }};

    (shl_arr $reg:ident, $idx: expr, $mb: expr, $val: expr) => {{
        $mb.ppu_mut().state.$reg[$idx] <<= $val
    }};
}
