fn main() {
    // ANCHOR: here
    {
        // s is valid from this point forward
        let s = String::from("hello");

        // do stuff with s
    // this scope is now over, and s is no
    }
                                       // longer valid
    // ANCHOR_END: here
}
