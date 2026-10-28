namespace OrderDesk.Core;

public sealed class Order
{
    public Order(string id, decimal total)
    {
        Id = id;
        Total = total;
    }

    public string Id { get; }

    public decimal Total { get; }
}
