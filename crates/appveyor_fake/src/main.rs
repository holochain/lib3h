fn stack_2() {
    panic!("get a backtrace");
}

fn stack_1() {
    stack_2();
}

fn main() {
    stack_1();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_exec_main() {
        main()
    }
}
