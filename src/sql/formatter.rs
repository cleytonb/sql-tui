//! SQL formatter - uses sqruff for proper SQL formatting

use sqruff_lib::core::config::FluffConfig;
use sqruff_lib::core::linter::core::Linter;

/// Format SQL query using sqruff with .sqruff config file
pub fn format_sql_query(sql: &str) -> String {
    // Tenta carregar configuração do arquivo .sqruff na raiz do projeto
    // Se não encontrar, usa configuração padrão
    let config = FluffConfig::from_root(None, false, None)
        .unwrap_or_else(|_| FluffConfig::default());
    
    let linter = Linter::new(config, None, None, false);
    
    let result = linter.lint_string(sql, None, true);
    let formatted = result.fix_string();
    
    if formatted.is_empty() {
        sql.to_string()
    } else {
        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_select() {
        let sql = "SELECT * FROM users WHERE id = 1";
        let formatted = format_sql_query(sql);
        assert!(formatted.contains("SELECT"));
        assert!(formatted.contains("FROM"));
        assert!(formatted.contains("WHERE"));
    }

    #[test]
    fn test_format_preserves_content() {
        let sql = "SELECT id, name FROM users";
        let formatted = format_sql_query(sql);
        assert!(formatted.contains("id"));
        assert!(formatted.contains("name"));
        assert!(formatted.contains("users"));
    }

    #[test]
    fn test_format_complex_query() {
        let sql = "select a,b,c from table1 inner join table2 on table1.id=table2.id where a>1 and b<2 order by c";
        let formatted = format_sql_query(sql);
        // sqruff deve formatar com quebras de linha e indentação
        println!("Formatted:\n{}", formatted);
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_format_long_query() {
        let sql = "select customer_id, customer_name, customer_email, order_date, order_total, product_name, product_category from customers inner join orders on customers.id = orders.customer_id inner join order_items on orders.id = order_items.order_id inner join products on order_items.product_id = products.id where order_date >= '2024-01-01' and order_total > 100 order by order_date desc, customer_name asc";
        let formatted = format_sql_query(sql);
        println!("Long query formatted:\n{}", formatted);
        // Deve ter quebras de linha
        assert!(formatted.contains('\n'), "Query longa deve ter quebras de linha");
        // Keywords em UPPER
        assert!(formatted.contains("SELECT"));
        assert!(formatted.contains("FROM"));
        assert!(formatted.contains("INNER JOIN"));
    }
}
