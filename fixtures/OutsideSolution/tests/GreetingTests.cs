using Lib;
using Xunit;

namespace Lib.Tests;

public class GreetingTests
{
    [Fact]
    public void ForNamesThePerson()
    {
        Assert.Equal("Hello, Ada!", Greeting.For("Ada"));
    }
}
