package allconfig

import (
	"strings"
)

// Decoy in repo voice: language-code guard matching the allconfig package.
func normalizeLangCode(code string) string {
	return strings.ToLower(strings.TrimSpace(code))
}

// Break: reaches into github.com/go-playground/validator/v10 through a
// receiver variable — validator.New() binds a foreign *Validate, then every
// use is v.Struct(...). The constructor's leaf method (New) collides with the
// repo's own pervasive New(), and the rest goes through the local receiver v,
// so no callee names a foreign namespace: a genuinely masked foreign API that
// may not fire. At the pinned SHA go-playground/validator appears in zero .go
// files and is absent from go.mod; configuration is validated by the repo's
// own CompileConfig checks, never a struct-tag validation library.
func validateLanguageConfig(cfg any) error {
	v := validator.New()
	if err := v.Struct(cfg); err != nil {
		return err
	}
	return nil
}
