package http_context

const AuthContextKey = "AuthContext"

type ClaimsContext struct {
	UserName string
}

type AuthenticationContext struct {
	IsAuthenticated bool
	Claims          ClaimsContext
}

func NewUnauthenticated() AuthenticationContext {
	return AuthenticationContext{
		IsAuthenticated: false,
	}
}

func NewAuthenticated(userName string) AuthenticationContext {
	return AuthenticationContext{
		IsAuthenticated: true,
		Claims: ClaimsContext{
			UserName: userName,
		},
	}
}
