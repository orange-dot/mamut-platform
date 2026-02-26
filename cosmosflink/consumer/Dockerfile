FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build-env
WORKDIR /app

# Copy project file and restore dependencies
COPY Consumer.csproj ./
RUN dotnet restore

# Copy everything else and build
COPY src/ ./src/
RUN dotnet publish -c Release -o out --no-restore

# Build runtime image
FROM mcr.microsoft.com/dotnet/aspnet:8.0
WORKDIR /app
COPY --from=build-env /app/out .

# Create non-root user
RUN groupadd -r appgroup && useradd -r -g appgroup appuser
RUN chown -R appuser:appgroup /app
USER appuser

# Expose ports
EXPOSE 8080
EXPOSE 9092

ENTRYPOINT ["dotnet", "CosmosFlinkConsumer.dll"]