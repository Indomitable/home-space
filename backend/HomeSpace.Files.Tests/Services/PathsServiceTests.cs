using HomeSpace.Files.Configuration;
using HomeSpace.Files.Services;
using Xunit;

namespace HomeSpace.Files.Tests.Services;

public class PathsServiceTests
{
    private const string BasePath = "/files";
    private readonly PathsService service;
    
    public PathsServiceTests()
    {
        var config = new FilesConfiguration { BaseLocation = BasePath };
        service = new PathsService(config);
    }

    [Fact]
    public void UserIdShouldBeUsedForUserDirectory()
    {
        Assert.Equal(BasePath + "/20", service.UserDirectory(20));
    }

    [Fact]
    public void ShouldBeAbleToResolveAbsolutePathFromRelative()
    {
        Assert.Equal(BasePath + "/10/test/abc", service.ResolveAbsolutePath(10, "/test/abc"));
    }
    
    [Fact]
    public void ShouldBeAbleToResolveRelativePathFromAbsolutePath()
    {
        Assert.Equal("/test/abc", service.ResolveRelativePath(10, "/files/10/test/abc"));
    }
}