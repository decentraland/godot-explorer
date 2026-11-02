pub trait FindNthChar {
    fn find_nth_char(&self, n: i32, character: char) -> Option<usize>;
}

impl FindNthChar for &str {
    fn find_nth_char(&self, mut n: i32, character: char) -> Option<usize> {
        for (i, c) in self.char_indices() {
            if c == character {
                n -= 1;
            }
            if n == 0 {
                return Some(i);
            }
        }

        None
    }
}

impl FindNthChar for String {
    fn find_nth_char(&self, n: i32, character: char) -> Option<usize> {
        self.as_str().find_nth_char(n, character)
    }
}
