fn main() {
    // ANCHOR: here
    let mut s = String::from("hello");

    // push_str() appends a literal to a String
    s.push_str(", world!");

    // this will print `hello, world!`
    println!("{s}");
    // ANCHOR_END: here
}
