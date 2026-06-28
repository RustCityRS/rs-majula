use rs_io::Packet;
use rs_io::jag::JagFile;

const PERIOD: &[u8] = b"dot";
const AMPERSAT: &[u8] = b"(a)";
const SLASH: &[u8] = b"slash";
const WHITELIST: &[&str] = &[
    "cook",
    "cook's",
    "cooks",
    "seeks",
    "sheet",
    #[cfg(since_244)]
    "woop",
    #[cfg(since_244)]
    "woops",
    #[cfg(since_254)]
    "faq",
];

type BadCombinations = Box<[Option<Box<[[i8; 2]]>>]>;
type WordList = Box<[Box<[u8]>]>;

pub struct WordEncProvider {
    pub bads: WordList,
    pub bad_combinations: BadCombinations,
    pub fragments: Box<[i32]>,
    pub tlds: WordList,
    pub tld_types: Box<[u8]>,
    pub domains: WordList,
}

impl WordEncProvider {
    pub fn from_jag(jag_bytes: &[u8]) -> WordEncProvider {
        let jag = JagFile::from(jag_bytes.to_vec());

        let (bads, bad_combinations) = jag
            .read("badenc.txt")
            .map(|p| decode_badenc(&p.data))
            .unwrap_or_default();

        let fragments = jag
            .read("fragmentsenc.txt")
            .map(|p| decode_fragments(&p.data))
            .unwrap_or_default();

        let (tlds, tld_types) = jag
            .read("tldlist.txt")
            .map(|p| decode_tldlist(&p.data))
            .unwrap_or_default();

        let domains = jag
            .read("domainenc.txt")
            .map(|p| decode_domains(&p.data))
            .unwrap_or_default();

        WordEncProvider {
            bads,
            bad_combinations,
            fragments: Box::from(fragments),
            tlds,
            tld_types: Box::from(tld_types),
            domains,
        }
    }

    pub fn filter(&self, input: &str) -> String {
        let mut chars: Vec<char> = input.chars().collect();
        format_chars(&mut chars);
        let trimmed: String = chars.into_iter().collect::<String>().trim().to_string();
        let lowercase = trimmed.to_lowercase();
        let mut filtered: Vec<char> = lowercase.chars().collect();

        self.filter_tlds(&mut filtered);
        self.filter_bad_words(&mut filtered);
        self.filter_domains(&mut filtered);
        self.filter_fragments(&mut filtered);

        for word in WHITELIST {
            let mut offset = 0;
            while let Some(pos) = lowercase[offset..].find(word) {
                let abs_pos = offset + pos;
                for (i, ch) in word.chars().enumerate() {
                    filtered[abs_pos + i] = ch;
                }
                offset = abs_pos + 1;
            }
        }

        let trimmed_chars: Vec<char> = trimmed.chars().collect();
        replace_uppercases(&mut filtered, &trimmed_chars);
        format_uppercases(&mut filtered);
        filtered.iter().collect::<String>().trim().to_string()
    }

    // --- bad words ---

    fn filter_bad_words(&self, chars: &mut [char]) {
        for _ in 0..2 {
            for i in (0..self.bads.len()).rev() {
                let combos = self.bad_combinations.get(i).and_then(|c| c.as_deref());
                self.filter_bad_combinations(combos, chars, &self.bads[i]);
            }
        }
    }

    pub fn filter_bad_combinations(
        &self,
        combos: Option<&[[i8; 2]]>,
        chars: &mut [char],
        bads: &[u8],
    ) {
        if bads.len() > chars.len() {
            return;
        }
        for start_index in 0..=chars.len() - bads.len() {
            let (current_index, bad_index, has_symbol, has_number, has_digit) =
                self.process_bad_characters(chars, bads, start_index);

            if !(bad_index >= bads.len() && (!has_number || !has_digit)) {
                continue;
            }

            let mut should_filter = true;

            if has_symbol {
                let is_before_symbol = start_index == 0
                    || (is_symbol(&chars[start_index - 1]) && chars[start_index - 1] != '\'');
                let is_after_symbol = current_index >= chars.len()
                    || (is_symbol(&chars[current_index]) && chars[current_index] != '\'');

                if !is_before_symbol || !is_after_symbol {
                    let mut is_substring_valid = false;
                    let mut local_index = if is_before_symbol {
                        start_index as i32
                    } else {
                        start_index as i32 - 2
                    };

                    while !is_substring_valid && (local_index as usize) < current_index {
                        if local_index >= 0 {
                            let li = local_index as usize;
                            if !is_symbol(&chars[li]) || chars[li] == '\'' {
                                let mut sub = ['\0'; 3];
                                let mut sub_len = 0;
                                while sub_len < 3
                                    && li + sub_len < chars.len()
                                    && (!is_symbol(&chars[li + sub_len])
                                        || chars[li + sub_len] == '\'')
                                {
                                    sub[sub_len] = chars[li + sub_len];
                                    sub_len += 1;
                                }
                                let mut is_valid = true;
                                if sub_len == 0 {
                                    is_valid = false;
                                }
                                if sub_len < 3
                                    && li >= 1
                                    && (!is_symbol(&chars[li - 1]) || chars[li - 1] == '\'')
                                {
                                    is_valid = false;
                                }
                                if is_valid && !self.is_bad_fragment(&sub[..sub_len]) {
                                    is_substring_valid = true;
                                }
                            }
                        }
                        local_index += 1;
                    }
                    if !is_substring_valid {
                        should_filter = false;
                    }
                }
            } else {
                let current_char = if start_index >= 1 {
                    chars[start_index - 1]
                } else {
                    ' '
                };
                let next_char = if current_index < chars.len() {
                    chars[current_index]
                } else {
                    ' '
                };
                let current = get_index(current_char);
                let next = get_index(next_char);
                if let Some(combos) = combos
                    && combo_matches(current, combos, next)
                {
                    should_filter = false;
                }
            }

            if !should_filter {
                continue;
            }

            let mut numeral_count = 0;
            let mut alpha_count = 0;
            for &c in chars.iter().take(current_index).skip(start_index) {
                if is_numerical(&c) {
                    numeral_count += 1;
                } else if is_alpha(&c) {
                    alpha_count += 1;
                }
            }
            if numeral_count <= alpha_count {
                mask_chars(start_index, current_index, chars);
            }
        }
    }

    fn process_bad_characters(
        &self,
        chars: &[char],
        bads: &[u8],
        start_index: usize,
    ) -> (usize, usize, bool, bool, bool) {
        let mut index = start_index;
        let mut bad_index: usize = 0;
        let mut count: usize = 0;
        let mut has_symbol = false;
        let mut has_number = false;
        let mut has_digit = false;

        while index < chars.len() && !(has_number && has_digit) {
            let current_char = chars[index];
            let next_char = if index + 1 < chars.len() {
                chars[index + 1]
            } else {
                '\0'
            };

            if bad_index < bads.len() {
                let current_length =
                    get_emulated_bad_char_len(next_char, bads[bad_index] as char, current_char);
                if current_length > 0 {
                    if current_length == 1 && is_numerical(&current_char) {
                        has_number = true;
                    }
                    if current_length == 2
                        && (is_numerical(&current_char) || is_numerical(&next_char))
                    {
                        has_number = true;
                    }
                    index += current_length;
                    bad_index += 1;
                    continue;
                }
            }

            if bad_index == 0 {
                break;
            }

            let previous_length =
                get_emulated_bad_char_len(next_char, bads[bad_index - 1] as char, current_char);
            if previous_length > 0 {
                index += previous_length;
            } else {
                if bad_index >= bads.len() || !is_not_lowercase_alpha(&current_char) {
                    break;
                }
                if is_symbol(&current_char) && current_char != '\'' {
                    has_symbol = true;
                }
                if is_numerical(&current_char) {
                    has_digit = true;
                }
                index += 1;
                count += 1;
                if index > start_index && (count * 100 / (index - start_index)) > 90 {
                    break;
                }
            }
        }

        (index, bad_index, has_symbol, has_number, has_digit)
    }

    // --- domains ---

    fn filter_domains(&self, chars: &mut [char]) {
        let mut ampersat = chars.to_vec();
        let mut period = chars.to_vec();
        self.filter_bad_combinations(None, &mut ampersat, AMPERSAT);
        self.filter_bad_combinations(None, &mut period, PERIOD);

        for i in (0..self.domains.len()).rev() {
            self.filter_domain(&period, &ampersat, &self.domains[i], chars);
        }
    }

    fn filter_domain(&self, period: &[char], ampersat: &[char], domain: &[u8], chars: &mut [char]) {
        let domain_length = domain.len();
        if domain_length > chars.len() {
            return;
        }
        for index in 0..=chars.len() - domain_length {
            let (matched, current_index) = find_matching_domain(index, domain, chars);
            if !matched {
                continue;
            }
            let ampersat_status = prefix_symbol_status(index, chars, 3, ampersat, &['@']);
            let period_status =
                suffix_symbol_status(current_index - 1, chars, 3, period, &['.', ',']);
            if ampersat_status > 2 || period_status > 2 {
                mask_chars(index, current_index, chars);
            }
        }
    }

    // --- tlds ---

    fn filter_tlds(&self, chars: &mut [char]) {
        let mut period = chars.to_vec();
        let mut slash = chars.to_vec();
        self.filter_bad_combinations(None, &mut period, PERIOD);
        self.filter_bad_combinations(None, &mut slash, SLASH);

        for i in 0..self.tlds.len() {
            self.filter_tld(&slash, self.tld_types[i], chars, &self.tlds[i], &period);
        }
    }

    fn filter_tld(
        &self,
        slash: &[char],
        tld_type: u8,
        chars: &mut [char],
        tld: &[u8],
        period: &[char],
    ) {
        if tld.len() > chars.len() {
            return;
        }
        for index in 0..=chars.len() - tld.len() {
            let (current_index, tld_index) = process_tlds(chars, tld, index);
            if tld_index < tld.len() {
                continue;
            }

            let period_filter_status = prefix_symbol_status(index, chars, 3, period, &[',', '.']);
            let slash_filter_status =
                suffix_symbol_status(current_index - 1, chars, 5, slash, &['\\', '/']);

            let should_filter = match tld_type {
                1 => period_filter_status > 0 && slash_filter_status > 0,
                2 => {
                    (period_filter_status > 2 && slash_filter_status > 0)
                        || (period_filter_status > 0 && slash_filter_status > 2)
                }
                3 => period_filter_status > 0 && slash_filter_status > 2,
                _ => false,
            };

            if !should_filter {
                continue;
            }

            let mut start_filter = index;
            let mut end_filter = current_index - 1;

            if period_filter_status > 2 {
                if period_filter_status == 4 {
                    let mut found = false;
                    let mut pi = start_filter as i32 - 1;
                    while pi >= 0 {
                        if found {
                            if period[pi as usize] != '*' {
                                break;
                            }
                            start_filter = pi as usize;
                        } else if period[pi as usize] == '*' {
                            start_filter = pi as usize;
                            found = true;
                        }
                        pi -= 1;
                    }
                }
                let mut found = false;
                let mut pi = start_filter as i32 - 1;
                while pi >= 0 {
                    if found {
                        if is_symbol(&chars[pi as usize]) {
                            break;
                        }
                        start_filter = pi as usize;
                    } else if !is_symbol(&chars[pi as usize]) {
                        found = true;
                        start_filter = pi as usize;
                    }
                    pi -= 1;
                }
            }

            if slash_filter_status > 2 {
                if slash_filter_status == 4 {
                    let mut found = false;
                    for (pi, &c) in slash
                        .iter()
                        .enumerate()
                        .take(chars.len())
                        .skip(end_filter + 1)
                    {
                        if found {
                            if c != '*' {
                                break;
                            }
                            end_filter = pi;
                        } else if c == '*' {
                            end_filter = pi;
                            found = true;
                        }
                    }
                }
                let mut found = false;
                for (pi, &c) in chars.iter().enumerate().skip(end_filter + 1) {
                    if found {
                        if is_symbol(&c) {
                            break;
                        }
                        end_filter = pi;
                    } else if !is_symbol(&c) {
                        found = true;
                        end_filter = pi;
                    }
                }
            }

            mask_chars(start_filter, end_filter + 1, chars);
        }
    }

    // --- fragments ---

    fn filter_fragments(&self, chars: &mut [char]) {
        let mut current_index: usize = 0;
        let mut start_index: usize = 0;
        loop {
            let number_index = index_of_number(chars, current_index);
            if number_index == usize::MAX {
                return;
            }

            let mut is_symbol_or_not_lowercase_alpha = false;
            for &c in chars.iter().take(number_index).skip(current_index) {
                if !is_symbol(&c) && !is_not_lowercase_alpha(&c) {
                    is_symbol_or_not_lowercase_alpha = true;
                }
            }

            if is_symbol_or_not_lowercase_alpha {
                start_index = 0;
            }
            if start_index == 0 {
                start_index = 1;
                // current_index = number_index;
            }

            let end = index_of_non_number(number_index, chars);
            let mut value: i32 = 0;
            for &c in chars.iter().take(end).skip(number_index) {
                value = value * 10 + (c as i32 - 48);
            }

            if value <= 255 && end - number_index <= 8 {
                start_index += 1;
            } else {
                start_index = 0;
            }

            if start_index == 4 {
                mask_chars(number_index, end, chars);
                start_index = 0;
            }
            current_index = end;
        }
    }

    fn is_bad_fragment(&self, chars: &[char]) -> bool {
        if is_numerical_chars(chars) {
            return true;
        }
        let value = get_fragment_integer(chars);

        if self.fragments.is_empty() {
            return false;
        }
        if value == self.fragments[0] || value == self.fragments[self.fragments.len() - 1] {
            return true;
        }

        let mut start: usize = 0;
        let mut end = self.fragments.len() - 1;
        while start <= end {
            let mid = (start + end) / 2;
            if value == self.fragments[mid] {
                return true;
            } else if value < self.fragments[mid] {
                if mid == 0 {
                    break;
                }
                end = mid - 1;
            } else {
                start = mid + 1;
            }
        }
        false
    }
}

// --- character classification helpers ---

fn is_symbol(c: &char) -> bool {
    !is_alpha(c) && !is_numerical(c)
}

fn is_not_lowercase_alpha(c: &char) -> bool {
    if is_lowercase_alpha(c) {
        matches!(c, 'v' | 'x' | 'j' | 'q' | 'z')
    } else {
        true
    }
}

fn is_alpha(c: &char) -> bool {
    is_lowercase_alpha(c) || is_uppercase_alpha(c)
}

fn is_numerical(c: &char) -> bool {
    c.is_ascii_digit()
}

fn is_lowercase_alpha(c: &char) -> bool {
    c.is_ascii_lowercase()
}

fn is_uppercase_alpha(c: &char) -> bool {
    c.is_ascii_uppercase()
}

fn is_numerical_chars(chars: &[char]) -> bool {
    for &c in chars {
        if !is_numerical(&c) && c != '\0' {
            return false;
        }
    }
    true
}

fn mask_chars(offset: usize, length: usize, chars: &mut [char]) {
    for c in &mut chars[offset..length] {
        *c = '*';
    }
}

fn masked_count_backwards(chars: &[char], offset: usize) -> usize {
    let mut count = 0;
    let mut i = offset as i32 - 1;
    while i >= 0 && is_symbol(&chars[i as usize]) {
        if chars[i as usize] == '*' {
            count += 1;
        }
        i -= 1;
    }
    count
}

fn masked_count_forwards(chars: &[char], offset: usize) -> usize {
    let mut count = 0;
    for &c in chars.iter().skip(offset + 1) {
        if !is_symbol(&c) {
            break;
        }
        if c == '*' {
            count += 1;
        }
    }
    count
}

fn masked_chars_status(
    chars: &[char],
    filtered: &[char],
    offset: usize,
    length: usize,
    prefix: bool,
) -> i32 {
    let count = if prefix {
        masked_count_backwards(filtered, offset)
    } else {
        masked_count_forwards(filtered, offset)
    };
    if count >= length {
        4
    } else if prefix {
        if offset >= 1 && is_symbol(&chars[offset - 1]) {
            1
        } else {
            0
        }
    } else if offset + 1 < chars.len() && is_symbol(&chars[offset + 1]) {
        1
    } else {
        0
    }
}

fn prefix_symbol_status(
    offset: usize,
    chars: &[char],
    length: usize,
    symbol_chars: &[char],
    symbols: &[char],
) -> i32 {
    if offset == 0 {
        return 2;
    }
    let mut i = offset as i32 - 1;
    while i >= 0 && is_symbol(&chars[i as usize]) {
        if symbols.contains(&chars[i as usize]) {
            return 3;
        }
        i -= 1;
    }
    masked_chars_status(chars, symbol_chars, offset, length, true)
}

fn suffix_symbol_status(
    offset: usize,
    chars: &[char],
    length: usize,
    symbol_chars: &[char],
    symbols: &[char],
) -> i32 {
    if offset + 1 == chars.len() {
        return 2;
    }
    for &c in chars.iter().skip(offset + 1) {
        if !is_symbol(&c) {
            break;
        }
        if symbols.contains(&c) {
            return 3;
        }
    }
    masked_chars_status(chars, symbol_chars, offset, length, false)
}

fn get_index(c: char) -> i8 {
    if is_lowercase_alpha(&c) {
        (c as i8) - b'a' as i8 + 1
    } else if c == '\'' {
        28
    } else if is_numerical(&c) {
        (c as i8) - b'0' as i8 + 29
    } else {
        27
    }
}

fn combo_matches(current_index: i8, combos: &[[i8; 2]], next_index: i8) -> bool {
    let mut start: usize = 0;
    let mut end = combos.len().wrapping_sub(1);
    if combos.is_empty() {
        return false;
    }

    while start <= end {
        let mid = (start + end) / 2;
        if combos[mid][0] == current_index && combos[mid][1] == next_index {
            return true;
        } else if current_index < combos[mid][0]
            || (current_index == combos[mid][0] && next_index < combos[mid][1])
        {
            if mid == 0 {
                break;
            }
            end = mid - 1;
        } else {
            start = mid + 1;
        }
    }
    false
}

fn get_emulated_bad_char_len(next_char: char, bad_char: char, current_char: char) -> usize {
    if bad_char == current_char {
        return 1;
    }
    if ('a'..='m').contains(&bad_char) {
        match bad_char {
            'a' => {
                if matches!(current_char, '4' | '@' | '^') {
                    return 1;
                }
                if current_char == '/' && next_char == '\\' {
                    return 2;
                }
            }
            'b' => {
                if matches!(current_char, '6' | '8') {
                    return 1;
                }
                if current_char == '1' && next_char == '3' {
                    return 2;
                }
            }
            'c' => {
                if matches!(current_char, '(' | '<' | '{' | '[') {
                    return 1;
                }
            }
            'd' if current_char == '[' && next_char == ')' => {
                return 2;
            }
            'e' if current_char == '3' || current_char == '€' => {
                return 1;
            }
            'f' => {
                if current_char == 'p' && next_char == 'h' {
                    return 2;
                }
                if current_char == '£' {
                    return 1;
                }
            }
            'g' => {
                if matches!(current_char, '9' | '6') {
                    return 1;
                }
            }
            'h' if current_char == '#' => {
                return 1;
            }
            'i' => {
                if matches!(current_char, 'y' | 'l' | 'j' | '1' | '!' | ':' | ';' | '|') {
                    return 1;
                }
            }
            'l' => {
                if matches!(current_char, '1' | '|' | 'i') {
                    return 1;
                }
            }
            _ => {}
        }
        return 0;
    }
    if ('n'..='z').contains(&bad_char) {
        match bad_char {
            'o' => {
                if matches!(current_char, '0' | '*') {
                    return 1;
                }
                if (current_char == '(' && next_char == ')')
                    || (current_char == '[' && next_char == ']')
                    || (current_char == '{' && next_char == '}')
                    || (current_char == '<' && next_char == '>')
                {
                    return 2;
                }
            }
            's' => {
                if matches!(current_char, '5' | 'z' | '$' | '2') {
                    return 1;
                }
            }
            't' => {
                if matches!(current_char, '7' | '+') {
                    return 1;
                }
            }
            'u' => {
                if current_char == 'v' {
                    return 1;
                }
                if !(current_char != '\\' || next_char != '/' && next_char != '|')
                    || (current_char == '|' && next_char == '/')
                {
                    return 2;
                }
            }
            'v' if (current_char == '\\' && (next_char == '/' || next_char == '|'))
                || (current_char == '|' && next_char == '/') =>
            {
                return 2;
            }
            'w' if current_char == 'v' && next_char == 'v' => {
                return 2;
            }
            'x' if (current_char == ')' && next_char == '(')
                || (current_char == '}' && next_char == '{')
                || (current_char == ']' && next_char == '[')
                || (current_char == '>' && next_char == '<') =>
            {
                return 2;
            }
            _ => {}
        }
        return 0;
    }
    if bad_char.is_ascii_digit() {
        if bad_char == '0' {
            if current_char == 'o' || current_char == 'O' {
                return 1;
            }
            if (current_char == '(' && next_char == ')')
                || (current_char == '{' && next_char == '}')
                || (current_char == '[' && next_char == ']')
            {
                return 2;
            }
        } else if bad_char == '1' && current_char == 'l' {
            return 1;
        }
        return 0;
    }
    if bad_char == ',' && current_char == '.' {
        return 1;
    }
    if bad_char == '.' && current_char == ',' {
        return 1;
    }
    if bad_char == '!' && current_char == 'i' {
        return 1;
    }
    0
}

fn get_emulated_domain_char_len(next_char: char, domain_char: char, current_char: char) -> usize {
    if domain_char == current_char {
        return 1;
    }
    if domain_char == 'o' && current_char == '0' {
        return 1;
    }
    if domain_char == 'o' && current_char == '(' && next_char == ')' {
        return 2;
    }
    if domain_char == 'c' && matches!(current_char, '(' | '<' | '[') {
        return 1;
    }
    if domain_char == 'e' && current_char == '€' {
        return 1;
    }
    if domain_char == 's' && current_char == '$' {
        return 1;
    }
    if domain_char == 'l' && current_char == 'i' {
        return 1;
    }
    0
}

fn find_matching_domain(start_index: usize, domain: &[u8], chars: &[char]) -> (bool, usize) {
    let mut current_index = start_index;
    let mut domain_index: usize = 0;

    while current_index < chars.len() && domain_index < domain.len() {
        let current_char = chars[current_index];
        let next_char = if current_index + 1 < chars.len() {
            chars[current_index + 1]
        } else {
            '\0'
        };
        let current_length =
            get_emulated_domain_char_len(next_char, domain[domain_index] as char, current_char);

        if current_length > 0 {
            current_index += current_length;
            domain_index += 1;
        } else {
            if domain_index == 0 {
                break;
            }
            let previous_length = get_emulated_domain_char_len(
                next_char,
                domain[domain_index - 1] as char,
                current_char,
            );
            if previous_length > 0 {
                current_index += previous_length;
            } else {
                if !is_symbol(&current_char) {
                    break;
                }
                current_index += 1;
            }
        }
    }
    (domain_index >= domain.len(), current_index)
}

fn process_tlds(chars: &[char], tld: &[u8], mut current_index: usize) -> (usize, usize) {
    let mut tld_index: usize = 0;
    while current_index < chars.len() && tld_index < tld.len() {
        let current_char = chars[current_index];
        let next_char = if current_index + 1 < chars.len() {
            chars[current_index + 1]
        } else {
            '\0'
        };
        let current_length =
            get_emulated_domain_char_len(next_char, tld[tld_index] as char, current_char);

        if current_length > 0 {
            current_index += current_length;
            tld_index += 1;
        } else {
            if tld_index == 0 {
                break;
            }
            let previous_length =
                get_emulated_domain_char_len(next_char, tld[tld_index - 1] as char, current_char);
            if previous_length > 0 {
                current_index += previous_length;
            } else {
                if !is_symbol(&current_char) {
                    break;
                }
                current_index += 1;
            }
        }
    }
    (current_index, tld_index)
}

fn index_of_number(chars: &[char], offset: usize) -> usize {
    for (i, &c) in chars.iter().enumerate().skip(offset) {
        if is_numerical(&c) {
            return i;
        }
    }
    usize::MAX
}

fn index_of_non_number(offset: usize, chars: &[char]) -> usize {
    for (i, &c) in chars.iter().enumerate().skip(offset) {
        if !is_numerical(&c) {
            return i;
        }
    }
    chars.len()
}

fn get_fragment_integer(chars: &[char]) -> i32 {
    if chars.len() > 6 {
        return 0;
    }
    let mut value: i32 = 0;
    for i in 0..chars.len() {
        let c = chars[chars.len() - i - 1];
        if is_lowercase_alpha(&c) {
            value = value * 38 + (c as i32) - ('a' as i32) + 1;
        } else if c == '\'' {
            value = value * 38 + 27;
        } else if is_numerical(&c) {
            value = value * 38 + (c as i32) - ('0' as i32) + 28;
        } else if c != '\0' {
            return 0;
        }
    }
    value
}

// --- formatting ---

fn format_chars(chars: &mut [char]) {
    let mut pos = 0;
    for i in 0..chars.len() {
        if is_character_allowed(chars[i]) {
            chars[pos] = chars[i];
        } else {
            chars[pos] = ' ';
        }
        if pos == 0 || chars[pos] != ' ' || chars[pos - 1] != ' ' {
            pos += 1;
        }
    }
    for c in chars.iter_mut().skip(pos) {
        *c = ' ';
    }
}

fn is_character_allowed(c: char) -> bool {
    (' '..='\x7f').contains(&c) || c == ' ' || c == '\n' || c == '\t' || c == '£' || c == '€'
}

fn replace_uppercases(chars: &mut [char], comparison: &[char]) {
    for i in 0..comparison.len().min(chars.len()) {
        if chars[i] != '*' && is_uppercase_alpha(&comparison[i]) {
            chars[i] = comparison[i];
        }
    }
}

fn format_uppercases(chars: &mut [char]) {
    let mut flagged = true;
    for c in chars.iter_mut() {
        if !is_alpha(c) {
            flagged = true;
        } else if flagged {
            if is_lowercase_alpha(c) {
                flagged = false;
            }
        } else if is_uppercase_alpha(c) {
            *c = (*c as u8 + b'a' - 65) as char;
        }
    }
}

// --- decoding ---

fn decode_badenc(data: &[u8]) -> (WordList, BadCombinations) {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    let mut bads = Vec::with_capacity(count);
    let mut combos = Vec::with_capacity(count);

    for _ in 0..count {
        let word_len = buf.g1() as usize;
        let word = (0..word_len)
            .map(|_| buf.g1())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        bads.push(word);

        let combo_count = buf.g1() as usize;
        let combo = (0..combo_count)
            .map(|_| [buf.g1s(), buf.g1s()])
            .collect::<Vec<_>>()
            .into_boxed_slice();
        combos.push(if combo.is_empty() { None } else { Some(combo) });
    }

    (bads.into_boxed_slice(), combos.into_boxed_slice())
}

fn decode_fragments(data: &[u8]) -> Vec<i32> {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    (0..count).map(|_| buf.g2() as i32).collect()
}

fn decode_tldlist(data: &[u8]) -> (WordList, Vec<u8>) {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    let mut tlds = Vec::with_capacity(count);
    let mut types = Vec::with_capacity(count);

    for _ in 0..count {
        types.push(buf.g1());
        let tld_len = buf.g1() as usize;
        tlds.push(
            (0..tld_len)
                .map(|_| buf.g1())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
    }

    (Box::from(tlds), types)
}

fn decode_domains(data: &[u8]) -> WordList {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    (0..count)
        .map(|_| {
            let len = buf.g1() as usize;
            (0..len)
                .map(|_| buf.g1())
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .collect()
}
