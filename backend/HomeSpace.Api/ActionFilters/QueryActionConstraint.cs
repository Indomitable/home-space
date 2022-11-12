using Microsoft.AspNetCore.Mvc.ActionConstraints;

namespace HomeSpace.Api.ActionFilters;

public class QueryActionConstraint : Attribute, IActionConstraint
{
    private readonly string requiredQueryKey;

    public QueryActionConstraint(string requiredQueryKey)
    {
        this.requiredQueryKey = requiredQueryKey;
    }
    
    public bool Accept(ActionConstraintContext context)
    {
        return context.RouteContext.HttpContext.Request.Query.ContainsKey(requiredQueryKey);
    }

    public int Order => 0;
} 