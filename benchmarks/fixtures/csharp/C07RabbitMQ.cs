using RabbitMQ.Client;

public class C07 {
    public IModel Channel(IConnection conn) {
        return conn.CreateModel();
    }
}
