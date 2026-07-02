using Npgsql;

public class C10 {
    public NpgsqlConnection Conn(string cs) {
        return new NpgsqlConnection(cs);
    }
}
