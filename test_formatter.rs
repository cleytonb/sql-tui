//! Script para testar o SQL formatter
//! 
//! Execute com: cargo run --bin test_formatter
//! 
//! Edite as queries abaixo para testar diferentes cenários

use sql_tui::sql::format_sql_query;

fn format_sql(sql: &str) -> String {
    format_sql_query(sql)
}

fn main() {
    println!("=== SQL Formatter Test ===\n");
    
    // Query 1: SELECT simples
    let query1 = "select * from users where id = 1";
    println!("Query 1 - SELECT simples:");
    println!("Input:  {}", query1);
    println!("Output:\n{}\n", format_sql(query1));
    
    // Query 2: SELECT com múltiplas colunas
    let query2 = "select id, name, email, created_at from customers where active = 1 order by name";
    println!("Query 2 - Múltiplas colunas:");
    println!("Input:  {}", query2);
    println!("Output:\n{}\n", format_sql(query2));
    
    // Query 3: JOIN
    let query3 = "select a.id, a.name, b.total from customers a join orders b on a.id = b.customer_id where b.total > 100";
    println!("Query 3 - JOIN:");
    println!("Input:  {}", query3);
    println!("Output:\n{}\n", format_sql(query3));
    
    // Query 4: Múltiplos JOINs
    let query4 = "select c.name, o.order_date, p.product_name, oi.quantity from customers c inner join orders o on c.id = o.customer_id inner join order_items oi on o.id = oi.order_id inner join products p on oi.product_id = p.id where o.order_date >= '2024-01-01'";
    println!("Query 4 - Múltiplos JOINs:");
    println!("Input:  {}", query4);
    println!("Output:\n{}\n", format_sql(query4));
    
    // Query 5: Subquery
    let query5 = "select * from users where id in (select user_id from orders where total > 1000)";
    println!("Query 5 - Subquery:");
    println!("Input:  {}", query5);
    println!("Output:\n{}\n", format_sql(query5));
    
    // Query 6: CTE
    let query6 = "with active_users as (select id, name from users where active = 1) select * from active_users where name like 'A%'";
    println!("Query 6 - CTE:");
    println!("Input:  {}", query6);
    println!("Output:\n{}\n", format_sql(query6));
    
    // Query 7: CASE WHEN
    let query7 = "select id, name, case when status = 1 then 'Active' when status = 2 then 'Inactive' else 'Unknown' end as status_text from users";
    println!("Query 7 - CASE WHEN:");
    println!("Input:  {}", query7);
    println!("Output:\n{}\n", format_sql(query7));
    
    // Query 8: GROUP BY com agregações
    let query8 = "select department, count(*) as total, sum(salary) as total_salary, avg(salary) as avg_salary from employees group by department having count(*) > 5 order by total desc";
    println!("Query 8 - GROUP BY:");
    println!("Input:  {}", query8);
    println!("Output:\n{}\n", format_sql(query8));
    
    // Query 9: UNION
    let query9 = "select id, name from customers union all select id, name from suppliers order by name";
    println!("Query 9 - UNION:");
    println!("Input:  {}", query9);
    println!("Output:\n{}\n", format_sql(query9));
    
    // Query 10: INSERT
    let query10 = "insert into users (name, email, created_at) values ('John', 'john@email.com', getdate())";
    println!("Query 10 - INSERT:");
    println!("Input:  {}", query10);
    println!("Output:\n{}\n", format_sql(query10));
    
    // Query 11: UPDATE
    let query11 = "update users set name = 'Jane', updated_at = getdate() where id = 1";
    println!("Query 11 - UPDATE:");
    println!("Input:  {}", query11);
    println!("Output:\n{}\n", format_sql(query11));
    
    // Query 12: DELETE
    let query12 = "delete from users where created_at < '2020-01-01' and active = 0";
    println!("Query 12 - DELETE:");
    println!("Input:  {}", query12);
    println!("Output:\n{}\n", format_sql(query12));
    
    println!("=== Fim dos testes ===");
}
