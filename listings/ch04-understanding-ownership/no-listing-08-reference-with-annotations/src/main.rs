fn main() {
    let s1 = String::from("hello");

    let len = calculate_length(&s1);

    println!("The length of '{s1}' is {len}.");
}

// ANCHOR: here
// s is a reference to a String
fn calculate_length(s: &String) -> usize {
    s.len()
// Here, s goes out of scope. But because s does not have ownership of what
}
  // it refers to, the String is not dropped.
// ANCHOR_END: here
