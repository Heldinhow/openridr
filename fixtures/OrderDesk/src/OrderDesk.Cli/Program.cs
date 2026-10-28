using OrderDesk.Core;

var order = new Order("A-104", 42m);
Console.WriteLine($"Order {order.Id}: {order.Total:C}");
