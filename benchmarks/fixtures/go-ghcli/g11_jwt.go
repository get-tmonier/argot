package cmd

import "github.com/golang-jwt/jwt/v5"

func signToken(secret []byte, sub string) (string, error) {
	tok := jwt.NewWithClaims(jwt.SigningMethodHS256, jwt.MapClaims{"sub": sub})
	return tok.SignedString(secret)
}
