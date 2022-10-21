using System.Text;
using Konscious.Security.Cryptography;
using Microsoft.Extensions.Options;

namespace HomeSpace.Security.Password;

public interface IPasswordHasher
{
    Task<PasswordHash> HashPassword(string password);
    Task<bool> VerifyHash(string password, PasswordHash hash);
}

internal sealed class PasswordHasher : IPasswordHasher
{
    private readonly PasswordConfiguration configuration;

    public PasswordHasher(PasswordConfiguration configuration)
    {
        this.configuration = configuration;
    }
    
    public async Task<PasswordHash> HashPassword(string password)
    {
        var buffer = new byte[64];
        new Random().NextBytes(buffer);
        var hash = await Hash(password, buffer);
        return new PasswordHash(hash, buffer);
    }

    public async Task<bool> VerifyHash(string password, PasswordHash hash)
    {
        var passwordHash = await Hash(password, hash.Salt);
        return passwordHash.SequenceEqual(hash.Password);
    }

    private Task<byte[]> Hash(string password, byte[] salt)
    {
        var passwordBytes = Encoding.UTF8.GetBytes(password);
        using var argon = new Argon2i(passwordBytes);
        argon.DegreeOfParallelism = configuration.DegreeOfParallelism;
        argon.MemorySize = configuration.MemorySize;
        argon.Iterations = configuration.Iterations;
        argon.AssociatedData = Encoding.UTF8.GetBytes(configuration.AssociatedData);
        argon.Salt = salt;
        return argon.GetBytesAsync(512);
    }
}