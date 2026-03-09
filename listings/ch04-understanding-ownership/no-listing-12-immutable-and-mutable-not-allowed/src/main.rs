fn main() {
    // ANCHOR: here
    let mut s = String::from("hello");

    // no problem
    let r1 = &s;
    // no problem
    let r2 = &s;
    // BIG PROBLEM
    let r3 = &mut s;

    println!("{r1}, {r2}, and {r3}");
    // ANCHOR_END: here
}
