pub const FILE_NAME: &str = "stage_order.toml";
pub const MAX_ORDER: u16 = 255;

pub fn parse(text: &str) -> (Vec<(String, u16)>, Vec<String>) {
    let mut orders: Vec<(String, u16)> = Vec::new();
    let mut problems = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            problems.push(format!("line {}: not ui_stage_id = position; ignored", index + 1));
            continue;
        };
        let key = key.trim().trim_matches('"').to_ascii_lowercase();
        if !key.starts_with("ui_stage_") {
            problems.push(format!("line {}: {key} is not a ui_stage_id; ignored", index + 1));
            continue;
        }
        let value = value.trim();
        let Some(order) = value.parse::<u16>().ok().filter(|order| *order <= MAX_ORDER) else {
            problems.push(format!("line {}: {value} is not a position from 0 to {MAX_ORDER}; ignored", index + 1));
            continue;
        };
        orders.retain(|(name, _)| *name != key);
        orders.push((key, order));
    }
    (orders, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_ids_and_positions() {
        let (orders, problems) = parse("# mine\nui_stage_one = 130\n\nUI_Stage_Two=255 # last\r\n\"ui_stage_three\" = 0\n");
        assert_eq!(
            orders,
            vec![
                ("ui_stage_one".to_owned(), 130),
                ("ui_stage_two".to_owned(), 255),
                ("ui_stage_three".to_owned(), 0)
            ]
        );
        assert!(problems.is_empty());
    }

    #[test]
    fn the_last_line_for_a_stage_wins() {
        let (orders, _) = parse("ui_stage_one = 130\nui_stage_one = 140\n");
        assert_eq!(orders, vec![("ui_stage_one".to_owned(), 140)]);
    }

    #[test]
    fn bad_lines_are_reported_and_leave_the_rest() {
        let (orders, problems) = parse("hello\nmario = 3\nui_stage_a = 256\nui_stage_b = -1\nui_stage_c = x\nui_stage_d = 9\n");
        assert_eq!(orders, vec![("ui_stage_d".to_owned(), 9)]);
        assert_eq!(problems.len(), 5);
        assert!(problems[0].starts_with("line 1"));
        assert!(problems[1].contains("not a ui_stage_id"));
        assert!(problems[2].contains("0 to 255"));
    }
}
