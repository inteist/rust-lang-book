use std::io;
use std::io::prelude::*;

fn main() {
    write_md(remove_hidden_lines(&read_md()));
}

fn read_md() -> String {
    let mut buffer = String::new();
    match io::stdin().read_to_string(&mut buffer) {
        Ok(_) => buffer,
        Err(error) => panic!("{error}"),
    }
}

fn write_md(output: String) {
    print!("{output}");
}

/// Removes mdbook hidden lines (lines starting with `# ` inside code blocks)
/// and automatically de-indents the visible body lines when those hidden lines
/// formed a wrapping scope such as `# fn main() {` / `# }`.
///
/// Each hidden line ending with `{` increases the de-dent level by 4 spaces;
/// each hidden line that is only `}` (or `};`, `},`) decreases it by 4.
/// Visible lines inside a code block have exactly `dedent` leading spaces
/// stripped before output, then the remaining indentation is converted from
/// 4-space to 2-space for horizontal compactness.
fn remove_hidden_lines(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut within_codeblock = false;
    let mut dedent: usize = 0;
    // Emit a '\n' *before* the next written line, so that skipped hidden
    // lines don't leave blank lines behind.
    let mut need_newline = false;

    for line in input.lines() {
        if !within_codeblock {
            if line.starts_with("```") {
                within_codeblock = true;
                dedent = 0;
            }
            if need_newline { out.push('\n'); }
            out.push_str(line);
            need_newline = true;
        } else {
            // Closing fence.
            if line.starts_with("```") {
                within_codeblock = false;
                dedent = 0;
                if need_newline { out.push('\n'); }
                out.push_str(line);
                need_newline = true;
                continue;
            }

            let is_hidden = line.starts_with("# ") || line == "#";
            if is_hidden {
                let content = if line == "#" { "" } else { &line[2..] };
                let trimmed = content.trim();
                if trimmed.ends_with('{') {
                    dedent += 4;
                } else if trimmed == "}" || trimmed == "};" || trimmed == "}," {
                    dedent = dedent.saturating_sub(4);
                }
                // Hidden lines are NOT emitted — don't push a '\n' for them.
            } else {
                let after_dedent = if dedent > 0 {
                    let leading = line.len() - line.trim_start_matches(' ').len();
                    let strip = dedent.min(leading);
                    &line[strip..]
                } else {
                    line
                };
                // Convert remaining 4-space indentation to 2-space
                let reindented = reindent_line(after_dedent, 4, 2);
                if need_newline { out.push('\n'); }
                out.push_str(&reindented);
                need_newline = true;
            }
        }
    }

    // Preserve trailing newline from the original.
    if input.ends_with('\n') {
        out.push('\n');
    }

    out
}

/// Convert leading indentation of a single line from `from_width` spaces
/// per level to `to_width` spaces per level.
fn reindent_line(line: &str, from_width: usize, to_width: usize) -> String {
    if from_width == to_width || from_width == 0 {
        return line.to_string();
    }
    let leading = line.len() - line.trim_start_matches(' ').len();
    let levels = leading / from_width;
    let remainder = leading % from_width;
    let new_indent = levels * to_width + remainder;
    let mut result = String::with_capacity(new_indent + line.len() - leading);
    for _ in 0..new_indent {
        result.push(' ');
    }
    result.push_str(&line[leading..]);
    result
}

#[cfg(test)]
mod tests {
    use crate::remove_hidden_lines;

    #[test]
    fn hidden_line_in_code_block_is_removed() {
        let input = "In this listing:\n\n```\nfn main() {\n# secret\n}\n```\n\nyou can see that...\n";
        let output = remove_hidden_lines(input);
        let desired = "In this listing:\n\n```\nfn main() {\n}\n```\n\nyou can see that...\n";
        assert_eq!(output, desired);
    }

    #[test]
    fn headings_arent_removed() {
        let input = "# Heading 1\n";
        let output = remove_hidden_lines(input);
        assert_eq!(output, "# Heading 1\n");
    }

    #[test]
    fn hidden_fn_main_wrapper_dedents_body() {
        // Standard pattern: code wrapped in a hidden fn main.
        let input = "```\n# fn main() {\n    let x = 5;\n    let y = x;\n# }\n```\n";
        let output = remove_hidden_lines(input);
        assert_eq!(output, "```\nlet x = 5;\nlet y = x;\n```\n");
    }

    #[test]
    fn visible_fn_main_body_is_reindented_to_2_spaces() {
        // When fn main is visible (not hidden), body indentation gets
        // converted from 4-space to 2-space for compactness.
        let input = "```\nfn main() {\n    let x = 5;\n}\n```\n";
        let output = remove_hidden_lines(input);
        assert_eq!(output, "```\nfn main() {\n  let x = 5;\n}\n```\n");
    }

    #[test]
    fn mixed_top_level_and_wrapped_code() {
        // `use` at top level before hidden fn main wrapper — only the wrapped
        // lines should be de-indented.
        let input = "```\nuse std::fs::File;\n\n# fn main() {\n    let f = File::open(\"x\");\n# }\n```\n";
        let output = remove_hidden_lines(input);
        assert_eq!(
            output,
            "```\nuse std::fs::File;\n\nlet f = File::open(\"x\");\n```\n"
        );
    }

    #[test]
    fn nested_hidden_scope_dedents_correctly() {
        // Two levels of hidden nesting should strip 8 spaces.
        let input =
            "```\n# fn outer() {\n#     fn inner() {\n        let x = 1;\n#     }\n# }\n```\n";
        let output = remove_hidden_lines(input);
        assert_eq!(output, "```\nlet x = 1;\n```\n");
    }

    #[test]
    fn reindent_converts_4_to_2() {
        use crate::reindent_line;
        assert_eq!(reindent_line("        double", 4, 2), "    double");
        assert_eq!(reindent_line("    single", 4, 2), "  single");
        assert_eq!(reindent_line("no indent", 4, 2), "no indent");
    }
}
