fn main() {
    // ANCHOR: here
    let mut s = String::from("hello");

    // no problem
    let r1 = &s;
    // no problem
    let r2 = &s;
    println!("{r1} and {r2}");
    // Variables r1 and r2 will not be used after this point.

    // no problem
    let r3 = &mut s;
    println!("{r3}");
    // ANCHOR_END: here
}
