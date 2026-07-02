using NLog;

public class C11 {
    private static readonly Logger log = LogManager.GetCurrentClassLogger();
    public void Run() { log.Info("run"); }
}
