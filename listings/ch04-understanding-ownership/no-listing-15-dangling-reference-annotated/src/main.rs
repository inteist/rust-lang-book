fn main() {
    let reference_to_nothing = dangle();
}

// ANCHOR: here
// dangle returns a reference to a String
fn dangle() -> &String {

    // s is a new String
    let s = String::from("hello");

    // we return a reference to the String, s
    &s
// Here, s goes out of scope and is dropped, so its memory goes away.
}
  // Danger!
  // ANCHOR_END: here
