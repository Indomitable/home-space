using System.Net.Mime;
using Microsoft.AspNetCore.Mvc.Formatters;

namespace HomeSpace.Api.Formatters;

public class TextInputFormatter: InputFormatter
{
    public TextInputFormatter()
    {
        SupportedMediaTypes.Add(MediaTypeNames.Text.Plain);
    }
    
    public override async Task<InputFormatterResult> ReadRequestBodyAsync(InputFormatterContext context)
    {
        using var reader = new StreamReader(context.HttpContext.Request.Body);
        var content = await reader.ReadToEndAsync();
        return await InputFormatterResult.SuccessAsync(content);
    }

    public override bool CanRead(InputFormatterContext context)
    {
        var requestContentType = context.HttpContext.Request.ContentType;
        return !string.IsNullOrEmpty(requestContentType) && requestContentType.StartsWith(MediaTypeNames.Text.Plain);
    }
}