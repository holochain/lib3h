use crate::Buffer;

impl Buffer for Vec<u8> {
    fn box_clone(&self) -> Box<dyn Buffer> {
        Box::new(self.clone())
    }
    fn as_buffer(&self) -> &dyn Buffer {
        &*self
    }
    fn as_buffer_mut(&mut self) -> &mut dyn Buffer {
        &mut *self
    }
    fn len(&self) -> usize {
        Vec::len(self)
    }
    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }
    fn set_no_access(&self) {}
    fn set_readable(&self) {}
    fn set_writable(&self) {}

    fn compare(&mut self, b: &mut Box<dyn Buffer>) -> i32 {
        let l = self.len();
        if l != b.len() {
            return 1;
        }
        let a = self.read_lock();
        let b = b.read_lock();
        let mut i = 0;
        while i < l {
            if a[i] != b[i] {
                return 1;
            }
            i = i+1;
        }
        return 0;
    }
}
