using AutoMapper;

public class C02 {
    public IMapper Build() {
        var cfg = new MapperConfiguration(c => { });
        return cfg.CreateMapper();
    }
}
