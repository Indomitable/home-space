package ioc

const (
	LifetimeSingleton = iota
	// LifetimeRequest
	LifetimeTransient
)

const CONTAINER_CONTEXT_KEY = "CONTAINER_CONTEXT"

type Registration interface {
	GetService() interface{}
}

type ClosableRegistration interface {
	Close()
}

type SingletonRegistration struct {
	singleton interface{}
}

func (r SingletonRegistration) GetService() interface{} {
	return r.singleton
}

type FactoryRegistration struct {
	factory func() interface{}
}

func (r FactoryRegistration) GetService() interface{} {
	return r.factory()
}

type Container interface {
	GetService(name string) interface{}
}

type ContainerImp struct {
	registrations map[string]Registration
	closable      []ClosableRegistration
}

func NewContainer() *ContainerImp {
	return &ContainerImp{
		registrations: make(map[string]Registration),
		closable:      make([]ClosableRegistration, 0),
	}
}

func (c *ContainerImp) GetService(name string) interface{} {
	registration := c.registrations[name]
	return registration.GetService()
}

func (c *ContainerImp) AddSingleton(name string, obj interface{}) {
	registration := SingletonRegistration{
		singleton: obj,
	}
	c.registrations[name] = registration
}

func (c *ContainerImp) AddTransient(name string, factory func() interface{}) {
	registration := FactoryRegistration{
		factory: factory,
	}
	c.registrations[name] = registration
}

func (c *ContainerImp) RegisterClosable(closable ClosableRegistration) {
	c.closable = append(c.closable, closable)
}

func (c *ContainerImp) Close() {
	for _, closable := range c.closable {
		closable.Close()
	}
}
