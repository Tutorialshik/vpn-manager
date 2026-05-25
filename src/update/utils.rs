pub fn sanitize_filename(s: &str) -> String {
    s.replace(['/', ':', '?', '&'], "_")
}
