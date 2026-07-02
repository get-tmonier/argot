use diesel::prelude::*;
pub fn count(conn: &mut PgConnection) -> i64 { use diesel::dsl::count_star; count_star().get_result(conn).unwrap() }
