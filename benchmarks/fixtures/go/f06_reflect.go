package cobra
import "reflect"
func fieldNames(v interface{}) []string {
	t := reflect.TypeOf(v); out := []string{}
	for i := 0; i < t.NumField(); i++ { out = append(out, t.Field(i).Name) }
	return out
}
