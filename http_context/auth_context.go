package http_context

const AuthContextKey = "AuthContext"

type ClaimsContext struct {
	UserId   int64
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

func NewAuthenticated(user_id int64, user_name string) AuthenticationContext {
	return AuthenticationContext{
		IsAuthenticated: true,
		Claims: ClaimsContext{
			UserId:   user_id,
			UserName: user_name,
		},
	}
}
