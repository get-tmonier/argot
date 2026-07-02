using StackExchange.Redis;

public class C08 {
    public IDatabase Db(ConnectionMultiplexer m) {
        return m.GetDatabase();
    }
}
