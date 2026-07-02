using Polly;

public class C05 {
    public void Retry() {
        Policy.Handle<System.Exception>().Retry(3).Execute(() => { });
    }
}
