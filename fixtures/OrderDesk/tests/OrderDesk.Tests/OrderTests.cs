using OrderDesk.Core;
using Xunit;

namespace OrderDesk.Tests;

public sealed class OrderTests
{
    [Fact]
    public void Id_is_preserved()
    {
        var order = new Order("A-104", 42m);

        Assert.Equal("A-104", order.Id);
    }

    [Fact]
    public void Total_is_preserved()
    {
        var order = new Order("A-104", 42m);

        Assert.Equal(42m, order.Total);
    }

    [Fact]
    public void Deliberate_failure_for_runner_evidence()
    {
        var order = new Order("A-104", 42m);

        Assert.Equal(43m, order.Total);
    }

    [Fact(Skip = "Fixture evidence: skipped on purpose until the fixture changes.")]
    public void Skipped_until_fixture_changes()
    {
    }
}
