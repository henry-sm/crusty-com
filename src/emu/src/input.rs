pub struct Input {
    // Controller state
    buttons: u16,
}

impl Input {
    pub fn new() -> Self {
        Input {
            buttons: 0,
        }
    }
    
    pub fn set_button(&mut self, button: u16, pressed: bool) {
        if pressed {
            self.buttons |= button;
        } else {
            self.buttons &= !button;
        }
    }
}