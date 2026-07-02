using Nancy;

public class C12 : NancyModule {
    public C12() {
        Get("/", args => "hello");
    }
}
