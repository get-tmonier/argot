using RestSharp;

public class C04 {
    public RestResponse Get(string url) {
        var client = new RestClient(url);
        return client.Execute(new RestRequest());
    }
}
