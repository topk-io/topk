use std::fmt;

use super::Image;

impl fmt::Display for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Image(size={}, mime_type={})",
            self.data.len(),
            self.mime_type
        )
    }
}
