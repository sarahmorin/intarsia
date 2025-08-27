/// Generate a parser for a given language
use crate::types::*;
use std::str::FromStr;

pub struct Parser<T>
where
    T: AST + FromStr,
{
    phantom: std::marker::PhantomData<T>,
}

/// Parser trait defines the behavior needed for parsing different AST nodes.
/// The FromStr trait in AST generally covers parsing operators from strings;
/// however, languages typically need to implement their own parsing for constants/terminals.
impl<T> Parser<T>
where
    T: AST + FromStr,
    T::Err: std::fmt::Display,
{
    /// Parse a string into an AST Expr node
    pub fn parse_expr(s: &str) -> Result<Expr<T>, String> {
        let trimmed = s.trim();

        // Check if this is a terminal (no parentheses)
        if !trimmed.contains('(') {
            // Terminal expression - just parse the operator
            let op = T::from_str(trimmed)
                .map_err(|e| format!("Failed to parse operator '{}': {}", trimmed, e))?;
            return Ok(Expr::new(op, vec![]));
        }

        // Find the first opening parenthesis
        let paren_pos = trimmed.find('(').ok_or("Expected '(' in expression")?;

        // Extract operator name
        let op_str = &trimmed[..paren_pos];
        let op = T::from_str(op_str)
            .map_err(|e| format!("Failed to parse operator '{}': {}", op_str, e))?;

        // Find matching closing parenthesis
        let args_str = &trimmed[paren_pos + 1..];
        if !args_str.ends_with(')') {
            return Err("Expected closing ')' in expression".to_string());
        }
        let args_str = &args_str[..args_str.len() - 1]; // Remove closing )

        // Parse arguments if any
        let mut args = Vec::new();
        if !args_str.trim().is_empty() {
            // Split by commas, but respect nested parentheses
            let arg_strings = Self::split_args(args_str)?;
            for arg_str in arg_strings {
                let arg_expr = Self::parse_expr(&arg_str)?;
                args.push(arg_expr);
            }
        }

        Ok(Expr::new(op, args))
    }

    /// Helper function to split arguments by commas while respecting nested parentheses
    fn split_args(s: &str) -> Result<Vec<String>, String> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut paren_depth = 0;
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '(' => {
                    paren_depth += 1;
                    current_arg.push(ch);
                }
                ')' => {
                    paren_depth -= 1;
                    current_arg.push(ch);
                }
                ',' if paren_depth == 0 => {
                    // Found a top-level comma, end current argument
                    args.push(current_arg.trim().to_string());
                    current_arg.clear();
                }
                _ => {
                    current_arg.push(ch);
                }
            }
        }

        // Add the last argument
        if !current_arg.trim().is_empty() {
            args.push(current_arg.trim().to_string());
        }

        if paren_depth != 0 {
            return Err("Mismatched parentheses in arguments".to_string());
        }

        Ok(args)
    }
    /// Parse a string into an AST Pattern node
    pub fn parse_pattern(s: &str) -> Result<Pattern<T>, String> {
        let trimmed = s.trim();

        // Check if this is a variable (starts with '?')
        if trimmed.starts_with('?') {
            // Variable - extract the variable name and create OpOrVar::Var
            let var_name = &trimmed[1..]; // Remove the '?' prefix
            if var_name.is_empty() {
                return Err("Variable name cannot be empty after '?'".to_string());
            }
            let op_or_var = OpOrVar::Var(var_name.to_string());
            return Ok(Expr::new(op_or_var, vec![]));
        }

        // Check if this is a terminal operator (no parentheses)
        if !trimmed.contains('(') {
            // Terminal expression - parse the operator and wrap in OpOrVar::Op
            let op = T::from_str(trimmed)
                .map_err(|e| format!("Failed to parse operator '{}': {}", trimmed, e))?;
            let op_or_var = OpOrVar::Op(op);
            return Ok(Expr::new(op_or_var, vec![]));
        }

        // Find the first opening parenthesis
        let paren_pos = trimmed.find('(').ok_or("Expected '(' in pattern")?;

        // Extract operator name
        let op_str = &trimmed[..paren_pos];
        let op = T::from_str(op_str)
            .map_err(|e| format!("Failed to parse operator '{}': {}", op_str, e))?;
        let op_or_var = OpOrVar::Op(op);

        // Find matching closing parenthesis
        let args_str = &trimmed[paren_pos + 1..];
        if !args_str.ends_with(')') {
            return Err("Expected closing ')' in pattern".to_string());
        }
        let args_str = &args_str[..args_str.len() - 1]; // Remove closing )

        // Parse arguments if any
        let mut args = Vec::new();
        if !args_str.trim().is_empty() {
            // Split by commas, but respect nested parentheses
            let arg_strings = Self::split_args(args_str)?;
            for arg_str in arg_strings {
                let arg_pattern = Self::parse_pattern(&arg_str)?;
                args.push(arg_pattern);
            }
        }

        Ok(Expr::new(op_or_var, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testlang::Ops;

    #[test]
    fn test_parse_terminal() {
        let result = Parser::<Ops>::parse_expr("true");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::ConstBool(true));
        assert!(expr.args().is_empty());
    }

    #[test]
    fn test_parse_simple_expression() {
        let result = Parser::<Ops>::parse_expr("Not(true)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::Not);
        assert_eq!(expr.args().len(), 1);
        assert_eq!(*expr.args()[0].op(), Ops::ConstBool(true));
    }

    #[test]
    fn test_parse_nested_expression() {
        let result = Parser::<Ops>::parse_expr("And(Not(true), false)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::And);
        assert_eq!(expr.args().len(), 2);

        // First argument: Not(true)
        let first_arg = &expr.args()[0];
        assert_eq!(*first_arg.op(), Ops::Not);
        assert_eq!(first_arg.args().len(), 1);
        assert_eq!(*first_arg.args()[0].op(), Ops::ConstBool(true));

        // Second argument: false
        let second_arg = &expr.args()[1];
        assert_eq!(*second_arg.op(), Ops::ConstBool(false));
        assert!(second_arg.args().is_empty());
    }

    #[test]
    fn test_parse_empty_args() {
        let result = Parser::<Ops>::parse_expr("And()");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::And);
        assert!(expr.args().is_empty());
    }

    #[test]
    fn test_parse_complex_nested() {
        let result = Parser::<Ops>::parse_expr("Or(And(Not(true), false), Not(And(true, false)))");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::Or);
        assert_eq!(expr.args().len(), 2);
    }

    #[test]
    fn test_parse_error_mismatched_parens() {
        let result = Parser::<Ops>::parse_expr("And(true, false");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unknown_operator() {
        // With the new FromStr implementation, unknown operators are parsed as string constants
        let result = Parser::<Ops>::parse_expr("UnknownOp(true)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        // The operator should be parsed as a string constant
        assert_eq!(*expr.op(), Ops::ConstStr("UnknownOp".to_string()));
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let result = Parser::<Ops>::parse_expr("  And(  Not( true ) , false  )  ");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::And);
        assert_eq!(expr.args().len(), 2);
    }

    #[test]
    fn test_parse_deeply_nested() {
        let result = Parser::<Ops>::parse_expr("And(Or(Not(true), false), Not(Or(false, true)))");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::And);
        assert_eq!(expr.args().len(), 2);

        // Check first argument: Or(Not(true), false)
        let first_arg = &expr.args()[0];
        assert_eq!(*first_arg.op(), Ops::Or);
        assert_eq!(first_arg.args().len(), 2);

        // Check second argument: Not(Or(false, true))
        let second_arg = &expr.args()[1];
        assert_eq!(*second_arg.op(), Ops::Not);
        assert_eq!(second_arg.args().len(), 1);
        assert_eq!(*second_arg.args()[0].op(), Ops::Or);
    }

    #[test]
    fn test_parse_pattern_variable() {
        let result = Parser::<Ops>::parse_pattern("?x");
        assert!(result.is_ok());
        let pattern = result.unwrap();
        match pattern.op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
        assert!(pattern.args().is_empty());
    }

    #[test]
    fn test_parse_pattern_terminal_operator() {
        let result = Parser::<Ops>::parse_pattern("true");
        assert!(result.is_ok());
        let pattern = result.unwrap();
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::ConstBool(true)),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }
        assert!(pattern.args().is_empty());
    }

    #[test]
    fn test_parse_pattern_with_variables() {
        let result = Parser::<Ops>::parse_pattern("And(?a, ?b)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::And),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        // Check arguments
        assert_eq!(pattern.args().len(), 2);

        // First argument should be variable 'a'
        match pattern.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "a"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }

        // Second argument should be variable 'b'
        match pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "b"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }

    #[test]
    fn test_parse_pattern_mixed() {
        let result = Parser::<Ops>::parse_pattern("And(Not(?x), true)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check top-level operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::And),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        assert_eq!(pattern.args().len(), 2);

        // First argument: Not(?x)
        let first_arg = &pattern.args()[0];
        match first_arg.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::Not),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }
        assert_eq!(first_arg.args().len(), 1);

        // The argument of Not should be variable 'x'
        match first_arg.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }

        // Second argument: true
        let second_arg = &pattern.args()[1];
        match second_arg.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::ConstBool(true)),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }
    }

    #[test]
    fn test_parse_pattern_empty_variable_error() {
        let result = Parser::<Ops>::parse_pattern("?");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Variable name cannot be empty"));
    }

    #[test]
    fn test_parse_pattern_complex_variables() {
        let result = Parser::<Ops>::parse_pattern("Or(And(?x, ?y), Not(?z))");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Should have Or at the top level
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::Or),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        assert_eq!(pattern.args().len(), 2);

        // First argument: And(?x, ?y)
        let first_arg = &pattern.args()[0];
        match first_arg.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::And),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }
        assert_eq!(first_arg.args().len(), 2);

        // Check variables x and y
        match first_arg.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
        match first_arg.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "y"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }

        // Second argument: Not(?z)
        let second_arg = &pattern.args()[1];
        match second_arg.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::Not),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }
        assert_eq!(second_arg.args().len(), 1);

        // Check variable z
        match second_arg.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "z"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }

    #[test]
    fn test_parse_pattern_whitespace_variables() {
        let result = Parser::<Ops>::parse_pattern("  And( ?a , ?b )  ");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::And),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        // Check arguments
        assert_eq!(pattern.args().len(), 2);

        // Both arguments should be variables despite whitespace
        match pattern.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "a"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }

        match pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "b"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }
}
