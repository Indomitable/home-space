namespace HomeSpace.Security.Password;

public record PasswordHash(byte[] Password, byte[] Salt);