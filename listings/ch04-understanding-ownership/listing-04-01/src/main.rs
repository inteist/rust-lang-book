fn main() {
    // ANCHOR: here
    // s is not valid here, since it's not yet declared
    {
        // s is valid from this point forward
        let s = "hello";

        // do stuff with s
    // this scope is now over, and s is no longer valid
    }
    // ANCHOR_END: here
}
