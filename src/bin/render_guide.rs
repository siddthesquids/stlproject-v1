use std::env;
use std::fs;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let source = args
        .first()
        .map(String::as_str)
        .unwrap_or("docs/stl-analyzer-guide.html");
    let target = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("STL-Analyzer-Project-Guide.pdf");
    let html = fs::read_to_string(source)?;
    write_pdf(Path::new(target), &wrap_lines(&html_to_text(&html), 92))?;
    println!("Wrote {target}");
    Ok(())
}

fn html_to_text(html: &str) -> String {
    let body = html
        .split("<body>")
        .nth(1)
        .unwrap_or(html)
        .split("</body>")
        .next()
        .unwrap_or(html);
    let mut output = String::new();
    let mut tag = String::new();
    let mut in_tag = false;
    for character in body.chars() {
        match character {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                let tag = tag.trim().to_ascii_lowercase();
                if matches!(
                    tag.as_str(),
                    "h1" | "h2"
                        | "h3"
                        | "p"
                        | "/p"
                        | "li"
                        | "tr"
                        | "/tr"
                        | "pre"
                        | "/pre"
                        | "div"
                        | "/div"
                        | "br"
                        | "br/"
                ) {
                    output.push('\n');
                }
                if tag == "li" {
                    output.push_str("- ");
                }
                if tag == "td" || tag == "th" {
                    output.push_str(" | ");
                }
                in_tag = false;
            }
            _ if in_tag => tag.push(character),
            _ => output.push(character),
        }
    }
    output
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    for original in text.lines() {
        let line = original.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            if !current.is_empty() && current.len() + word.len() + 1 > width {
                result.push(current);
                current = String::new();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.is_empty() {
            result.push(current);
        }
    }
    result
}

fn write_pdf(path: &Path, lines: &[String]) -> io::Result<()> {
    const LINES_PER_PAGE: usize = 54;
    let pages: Vec<_> = lines.chunks(LINES_PER_PAGE).collect();
    let mut objects = vec![String::new(), String::new(), String::new()];
    let kids = (0..pages.len())
        .map(|page| format!("{} 0 R", 4 + page * 2))
        .collect::<Vec<_>>()
        .join(" ");
    objects[0] = "<< /Type /Catalog /Pages 2 0 R >>".into();
    objects[1] = format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len());
    objects[2] = "<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>".into();
    for (page, page_lines) in pages.iter().enumerate() {
        let page_id = 4 + page * 2;
        let content_id = page_id + 1;
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"));
        let mut content = String::from("BT /F1 9 Tf 42 800 Td 13 TL ");
        for line in *page_lines {
            content.push_str(&format!("({}) Tj T* ", escape_pdf(line)));
        }
        content.push_str("ET");
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ));
    }
    let mut pdf = String::from("%PDF-1.4\n% Rust generated guide\n");
    let mut offsets = vec![0_usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
    }
    let xref = pdf.len();
    pdf.push_str(&format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    ));
    for offset in offsets.iter().skip(1) {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    ));
    fs::write(path, pdf)
}

fn escape_pdf(line: &str) -> String {
    line.chars()
        .map(|character| match character {
            '\\' => "\\\\".into(),
            '(' => "\\(".into(),
            ')' => "\\)".into(),
            value if value.is_ascii() => value.to_string(),
            _ => "?".into(),
        })
        .collect()
}
