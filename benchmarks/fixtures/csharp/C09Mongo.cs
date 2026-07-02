using MongoDB.Driver;

public class C09 {
    public IMongoDatabase Db(IMongoClient c) {
        return c.GetDatabase("app");
    }
}
