#![allow(unused_imports)]

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    And,
    Or,
    Not,
    LParen,
    RParen,
    Var(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoolExpr {
    Variable(String),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Not(Box<BoolExpr>),
}

#[derive(Debug, Clone)]
pub struct Decomposition {
    pub blocks: Vec<HashSet<String>>,
    pub template: String,
}

pub fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();
    let mut buffer = String::new();
    
    while let Some(&c) = chars.peek() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' => {
                buffer.push(c);
                chars.next();
                
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        buffer.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                
                match buffer.to_uppercase().as_str() {
                    "AND" => tokens.push(Token::And),
                    "OR" => tokens.push(Token::Or),
                    "NOT" => tokens.push(Token::Not),
                    _ => tokens.push(Token::Var(buffer.clone())),
                }
                buffer.clear();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            ' ' | '\t' | '\n' => {
                chars.next();
            }
            _ => {
                return Err(format!("Invalid character: {}", c));
            }
        }
    }
    
    Ok(tokens)
}

struct Parser {
    pub tokens: Vec<Token>,
    pub pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    pub fn consume(&mut self, expected: Token) -> Result<(), String> {
        if let Some(token) = self.peek() {
            if token == &expected {
                self.pos += 1;
                Ok(())
            } else {
                Err(format!("Expected {:?}, found {:?}", expected, token))
            }
        } else {
            Err("Unexpected end of input".to_string())
        }
    }

    pub fn parse_primary(&mut self) -> Result<BoolExpr, String> {
        match self.peek() {
            Some(Token::Var(name)) => {
                let name = name.clone();
                self.pos += 1;
                Ok(BoolExpr::Variable(name))
            }
            Some(Token::Not) => {
                self.pos += 1;
                let expr = self.parse_primary()?;
                Ok(BoolExpr::Not(Box::new(expr)))
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let expr = self.parse_expr()?;
                self.consume(Token::RParen)?;
                Ok(expr)
            }
            _ => Err("Expected variable, NOT, or (".to_string()),
        }
    }

    pub fn parse_and(&mut self) -> Result<BoolExpr, String> {
        let mut exprs = vec![self.parse_primary()?];

        while let Some(Token::And) = self.peek() {
            self.pos += 1;
            exprs.push(self.parse_primary()?);
        }

        if exprs.len() == 1 {
            Ok(exprs.remove(0))
        } else {
            Ok(BoolExpr::And(exprs))
        }
    }

    pub fn parse_expr(&mut self) -> Result<BoolExpr, String> {
        let mut exprs = vec![self.parse_and()?];

        while let Some(Token::Or) = self.peek() {
            self.pos += 1;
            exprs.push(self.parse_and()?);
        }

        if exprs.len() == 1 {
            Ok(exprs.remove(0))
        } else {
            Ok(BoolExpr::Or(exprs))
        }
    }

    pub fn parse(&mut self) -> Result<BoolExpr, String> {
        let expr = self.parse_expr()?;
        if self.pos < self.tokens.len() {
            return Err("Unexpected tokens at end".to_string());
        }
        Ok(expr)
    }
}

pub fn normalize_expr(expr: &BoolExpr) -> BoolExpr {
    match expr {
        BoolExpr::Or(exprs) => {
            let normalized: Vec<BoolExpr> = exprs.iter().map(normalize_expr).collect();
            BoolExpr::Or(normalized)
        }
        BoolExpr::And(exprs) => {
            let mut variables = Vec::new();
            let mut complex_exprs = Vec::new();
            
            for e in exprs {
                let normalized_e = normalize_expr(e);
                match normalized_e {
                    BoolExpr::Variable(_) => variables.push(normalized_e),
                    _ => complex_exprs.push(normalized_e),
                }
            }
            
            let mut all_exprs = variables;
            all_exprs.extend(complex_exprs);
            
            if all_exprs.len() == 1 {
                all_exprs.remove(0)
            } else {
                BoolExpr::And(all_exprs)
            }
        }
        BoolExpr::Not(inner) => {
            BoolExpr::Not(Box::new(normalize_expr(inner)))
        }
        BoolExpr::Variable(_) => expr.clone(),
    }
}

pub fn extract_blocks_from_normalized(expr: &BoolExpr) -> Vec<HashSet<String>> {
    let mut blocks = Vec::new();
    
    match expr {
        BoolExpr::Or(exprs) => {
            for e in exprs {
                match e {
                    BoolExpr::And(and_exprs) => {
                        let mut and_block = HashSet::new();
                        
                        for sub_expr in and_exprs {
                            match sub_expr {
                                BoolExpr::Variable(name) => {
                                    and_block.insert(name.clone());
                                }
                                BoolExpr::Not(inner) => {
                                    if !and_block.is_empty() {
                                        blocks.push(and_block);
                                        and_block = HashSet::new();
                                    }
                                    
                                    match &**inner {
                                        BoolExpr::Or(inner_exprs) => {
                                            for inner_e in inner_exprs {
                                                match inner_e {
                                                    BoolExpr::And(inner_and_exprs) => {
                                                        let mut inner_block = HashSet::new();
                                                        for inner_item in inner_and_exprs {
                                                            if let BoolExpr::Variable(name) = inner_item {
                                                                inner_block.insert(name.clone());
                                                            }
                                                        }
                                                        if !inner_block.is_empty() {
                                                            blocks.push(inner_block);
                                                        }
                                                    }
                                                    BoolExpr::Variable(name) => {
                                                        blocks.push(HashSet::from([name.clone()]));
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                        _ => {
                                            let nested = extract_blocks_from_normalized(inner);
                                            blocks.extend(nested);
                                        }
                                    }
                                }
                                _ => {
                                    if !and_block.is_empty() {
                                        blocks.push(and_block);
                                        and_block = HashSet::new();
                                    }
                                    let nested = extract_blocks_from_normalized(sub_expr);
                                    blocks.extend(nested);
                                }
                            }
                        }
                        
                        if !and_block.is_empty() {
                            blocks.push(and_block);
                        }
                    }
                    BoolExpr::Variable(name) => {
                        blocks.push(HashSet::from([name.clone()]));
                    }
                    _ => {
                        let nested = extract_blocks_from_normalized(e);
                        blocks.extend(nested);
                    }
                }
            }
        }
        BoolExpr::And(exprs) => {
            let mut and_block = HashSet::new();
            
            for e in exprs {
                match e {
                    BoolExpr::Variable(name) => {
                        and_block.insert(name.clone());
                    }
                    BoolExpr::Not(inner) => {
                        if !and_block.is_empty() {
                            blocks.push(and_block);
                            and_block = HashSet::new();
                        }
                        let nested = extract_blocks_from_normalized(inner);
                        blocks.extend(nested);
                    }
                    _ => {
                        if !and_block.is_empty() {
                            blocks.push(and_block);
                            and_block = HashSet::new();
                        }
                        let nested = extract_blocks_from_normalized(e);
                        blocks.extend(nested);
                    }
                }
            }
            
            if !and_block.is_empty() {
                blocks.push(and_block);
            }
        }
        BoolExpr::Not(inner) => {
            let nested = extract_blocks_from_normalized(inner);
            blocks.extend(nested);
        }
        BoolExpr::Variable(name) => {
            blocks.push(HashSet::from([name.clone()]));
        }
    }
    
    blocks
}

pub fn build_simplified_template_from_ast(expr: &BoolExpr) -> String {
    build_template_helper(expr, false)
}

pub fn build_template_helper(expr: &BoolExpr, in_and_context: bool) -> String {
    match expr {
        BoolExpr::Or(exprs) => {
            let parts: Vec<String> = exprs.iter()
                .map(|e| build_template_helper(e, false))
                .filter(|s| !s.is_empty())
                .collect();
            
            if parts.is_empty() {
                "".to_string()
            } else if parts.len() == 1 {
                let result = parts[0].clone();
                if in_and_context && result.contains(" OR ") {
                    format!("({})", result)
                } else {
                    result
                }
            } else {
                let result = parts.join(" OR ");
                if in_and_context {
                    format!("({})", result)
                } else {
                    result
                }
            }
        }
        BoolExpr::And(exprs) => {
            let mut parts = Vec::new();
            let mut has_variables = false;
            
            for e in exprs {
                match e {
                    BoolExpr::Variable(_) => {
                        if !has_variables {
                            parts.push("___".to_string());
                            has_variables = true;
                        }
                    }
                    _ => {
                        let part = build_template_helper(e, true);
                        if !part.is_empty() {
                            parts.push(part);
                        }
                    }
                }
            }
            
            if parts.is_empty() {
                "".to_string()
            } else if parts.len() == 1 {
                parts[0].clone()
            } else {
                parts.join(" AND ")
            }
        }
        BoolExpr::Not(inner) => {
            let inner_template = build_template_helper(inner, false);
            if inner_template.is_empty() {
                "".to_string()
            } else {
                format!("NOT({})", inner_template)
            }
        }
        BoolExpr::Variable(_) => {
            "___".to_string()
        }
    }
}

pub fn merge_adjacent_and(template: &str, blocks: &[HashSet<String>]) -> (String, Vec<HashSet<String>>) {
    let tokens: Vec<&str> = template.split_whitespace().collect();
    let mut new_template = String::new();
    let mut new_blocks = Vec::new();
    let mut block_idx = 0;
    let mut i = 0;
    
    let mut placeholder_positions = Vec::new();
    let temp_tokens: Vec<&str> = template.split_whitespace().collect();
    let mut pos = 0;
    
    for token in &temp_tokens {
        if *token == "___" {
            placeholder_positions.push(pos);
        }
        pos += 1;
    }
    
    while i < tokens.len() {
        if tokens[i] == "___" {
            if block_idx < blocks.len() {
                let mut should_merge = false;
                let mut merge_count = 1;
                let mut j = i;

                while j + 2 < tokens.len() && tokens[j] == "___" && tokens[j + 1] == "AND" && tokens[j + 2] == "___" {
                    should_merge = true;
                    merge_count += 1;
                    j += 2;
                }
                
                if should_merge {
                    let mut merged_block = blocks[block_idx].clone();
                    block_idx += 1;
                    
                    for _ in 1..merge_count {
                        if block_idx < blocks.len() {
                            for item in &blocks[block_idx] {
                                merged_block.insert(item.clone());
                            }
                            block_idx += 1;
                        }
                    }
                    
                    new_template.push_str("___");
                    new_blocks.push(merged_block);
                    i = j + 1;
                } else {
                    new_template.push_str("___");
                    new_blocks.push(blocks[block_idx].clone());
                    block_idx += 1;
                    i += 1;
                }
            } else {
                new_template.push_str("___");
                i += 1;
            }
        } else {
            if !new_template.ends_with(' ') && !new_template.is_empty() {
                new_template.push(' ');
            }
            new_template.push_str(tokens[i]);
            i += 1;
        }
        
        if i < tokens.len() && !new_template.ends_with(' ') {
            new_template.push(' ');
        }
    }
    
    while block_idx < blocks.len() {
        new_blocks.push(blocks[block_idx].clone());
        block_idx += 1;
    }
    
    (new_template.trim().to_string(), new_blocks)
}

pub fn decompose_query(expr_str: &str) -> Result<Decomposition, String> {
    let tokens = tokenize(expr_str)?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse()?;
    
    let normalized_expr = normalize_expr(&expr);
    
    let blocks = extract_blocks_from_normalized(&normalized_expr);
    
    let template = build_simplified_template_from_ast(&normalized_expr);
    
    let (final_template, final_blocks) = merge_adjacent_and(&template, &blocks);
    
    Ok(Decomposition {
        blocks: final_blocks,
        template: final_template,
    })
}

#[test]
#[ignore]
fn test_main() {
    let test_queries = vec![
        "Apple AND (NOT(Banana AND Cherry OR Date)) AND Elderberry AND Fig OR Grape AND Honeydew",
        
        "India AND Japan AND (NOT((Korea OR Laos) AND Malaysia)) OR Nepal AND Orange AND Peach",
        
        "W AND X AND (NOT(Y AND Z OR A AND B)) OR C AND D",
        
        "Alpha AND (Beta OR Gamma) AND NOT(Delta AND Epsilon) AND Zeta OR Eta AND Theta",
    
        "A AND B AND (NOT(C OR D AND E)) OR F AND G AND H",

        "(Alpha AND Beta) OR (Gamma AND (NOT(Delta OR Epsilon))) AND Zeta AND Eta",
        
        "(Alpha AND Beta) OR (Gamma AND Delta OR Epsilon) AND Zeta AND Eta",

        "NOT(I AND J) OR K AND L AND NOT(M OR N) AND O AND P",

        "X AND Y AND (Z OR A) AND NOT(B AND C OR D) OR E AND F",
    ];
    
    println!("=== Boolean Query Decomposition Test ===\n");
    
    for query in test_queries {
        println!("Original query: {}", query);
        
        match decompose_query(query) {
            Ok(decomp) => {
                print!("Set: {{");
                for (i, block) in decomp.blocks.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    
                    if block.len() == 1 {
                        print!("{}", block.iter().next().unwrap());
                    } else {
                        let mut items: Vec<&str> = block.iter().map(|s| s.as_str()).collect();
                        items.sort();
                        print!("{{{}}}", items.join(","));
                    }
                }
                println!("}}");
                println!("Template: {}", decomp.template);
                println!();
            }
            Err(e) => {
                println!("Error: {}", e);
                println!();
            }
        }
    }
}