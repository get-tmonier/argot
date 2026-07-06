package list

import (
	"fmt"
)

// Break: pulls in github.com/go-playground/validator/v10 to validate secret names via struct tags.
import (
	"github.com/go-playground/validator/v10"
)

type secretNameRule struct {
	Name string `validate:"required,alphanum,max=100"`
}

func validateSecretName(name string) error {
	v := validator.New()
	if err := v.Struct(secretNameRule{Name: name}); err != nil {
		return fmt.Errorf("invalid secret name: %w", err)
	}
	return nil
}

// Decoy in repo voice: plain visibility helper matching list.go style.
func normalizeVisibility(v string) string {
	if v == "" {
		return "all"
	}
	return v
}
