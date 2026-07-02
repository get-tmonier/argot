using MediatR;

public class C06 : IRequestHandler<string, int> {
    public System.Threading.Tasks.Task<int> Handle(string r, System.Threading.CancellationToken t) {
        return System.Threading.Tasks.Task.FromResult(0);
    }
}
