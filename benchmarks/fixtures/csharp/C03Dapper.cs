using Dapper;
using System.Data;

public class C03 {
    public int Count(IDbConnection db) {
        return db.QuerySingle<int>("select count(*) from users");
    }
}
