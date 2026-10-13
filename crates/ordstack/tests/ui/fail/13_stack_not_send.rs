use ordstack::Stack;
fn main() {
    let st = Stack::with_reserve(1 << 16);
    std::thread::spawn(move || drop(st)).join().unwrap(); // ERROR
}
