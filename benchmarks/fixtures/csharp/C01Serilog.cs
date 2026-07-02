using Serilog;

public class C01 {
    public void Run() {
        Log.Logger = new LoggerConfiguration().CreateLogger();
        Log.Information("run");
    }
}
