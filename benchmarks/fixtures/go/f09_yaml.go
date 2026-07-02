package cobra
import "gopkg.in/yaml.v2"
func parseSpec(data []byte) (map[string]string, error) { m := map[string]string{}; err := yaml.Unmarshal(data, &m); return m, err }
