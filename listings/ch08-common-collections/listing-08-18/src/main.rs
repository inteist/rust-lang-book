fn main() {
    // ANCHOR: here
    let s1 = String::from("Hello, ");
    let s2 = String::from("world!");
    // note s1 has been moved here and can no longer be used
    let s3 = s1 + &s2;
    // ANCHOR_END: here
}
