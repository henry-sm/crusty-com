/// Input handling for SNES controller simulation
/// Maps keyboard input to SNES joypad button states

#[derive(Debug, Clone, Copy)]
pub struct Input {
    /// Button states as a u8 bitmask
    /// Bit 7: B button
    /// Bit 6: Y button
    /// Bit 5: SELECT
    /// Bit 4: START
    /// Bit 3: D-Pad Up
    /// Bit 2: D-Pad Down
    /// Bit 1: D-Pad Left
    /// Bit 0: D-Pad Right
    pub buttons: u8,
    
    /// Shift register for serial joypad protocol (0x4217 reads)
    shift_register: u8,
    
    /// Strobe state for latching buttons
    strobe_latch: bool,
}

impl Input {
    pub fn new() -> Self {
        Input {
            buttons: 0xFF, // All buttons unpressed (inverted logic)
            shift_register: 0xFF,
            strobe_latch: false,
        }
    }
    
    /// Update button states from keyboard
    /// Each bit represents a button state (0 = pressed, 1 = unpressed)
    pub fn update_buttons(&mut self, buttons: u8) {
        self.buttons = buttons;
    }
    
    /// Set individual button state
    /// bit: button bit position (0-7)
    /// pressed: true if button is pressed
    pub fn set_button(&mut self, bit: usize, pressed: bool) {
        if bit < 8 {
            if pressed {
                self.buttons &= !(1 << bit);  // Clear bit (pressed)
            } else {
                self.buttons |= 1 << bit;     // Set bit (unpressed)
            }
        }
    }
    
    /// Handle strobe signal write to 0x4016
    pub fn write_strobe(&mut self, value: u8) {
        let new_strobe = (value & 0x01) != 0;
        
        // On falling edge of strobe (1 -> 0), latch button states into shift register
        if self.strobe_latch && !new_strobe {
            self.shift_register = self.buttons;
        }
        
        self.strobe_latch = new_strobe;
    }
    
    /// Read joypad data via serial protocol (0x4217)
    /// Returns one bit per read, shifted out in order: B, Y, SEL, STR, Up, Down, Left, Right
    pub fn read_serial(&self) -> u8 {
        // For now, return buttons directly (serial shifting would require mutable state)
        self.buttons
    }
    
    /// Read latched button state (0x4218/0x4219)
    pub fn read_buttons(&self) -> u8 {
        self.buttons
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_button_press() {
        let mut input = Input::new();
        assert_eq!(input.buttons, 0xFF); // All unpressed
        
        input.set_button(0, true);  // Press D-Pad Right
        assert_eq!(input.buttons, 0xFE); // Bit 0 cleared
    }
    
    #[test]
    fn test_strobe_and_shift() {
        let mut input = Input::new();
        input.buttons = 0b1001_0101; // Some buttons pressed
        
        // Strobe: 0 -> 1
        input.write_strobe(0x01);
        
        // Strobe: 1 -> 0 (falling edge latches)
        input.write_strobe(0x00);
        
        // Read bits in order
        assert_eq!(input.read_serial(), 0x01); // Bit 0 = 1
        assert_eq!(input.read_serial(), 0x00); // Bit 1 = 0
        assert_eq!(input.read_serial(), 0x01); // Bit 2 = 1
    }
}