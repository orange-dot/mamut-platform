name := "flink-cosmos-connector-scala"
version := "1.0-SNAPSHOT"
scalaVersion := "2.12.18"

val flinkVersion = "1.18.1"
val cosmosVersion = "4.54.0"
val jacksonVersion = "2.15.2"

libraryDependencies ++= Seq(
  // Flink Core Dependencies
  "org.apache.flink" %% "flink-streaming-scala" % flinkVersion % Provided,
  "org.apache.flink" % "flink-connector-base" % flinkVersion % Provided,
  "org.apache.flink" % "flink-clients" % flinkVersion % Provided,
  
  // Azure Cosmos DB
  "com.azure" % "azure-cosmos" % cosmosVersion,
  
  // JSON Processing
  "com.fasterxml.jackson.core" % "jackson-core" % jacksonVersion,
  "com.fasterxml.jackson.core" % "jackson-databind" % jacksonVersion,
  "com.fasterxml.jackson.datatype" % "jackson-datatype-jsr310" % jacksonVersion,
  "com.fasterxml.jackson.module" %% "jackson-module-scala" % jacksonVersion,
  
  // Logging
  "org.apache.logging.log4j" % "log4j-api" % "2.20.0",
  
  // Testing
  "org.scalatest" %% "scalatest" % "3.2.17" % Test,
  "org.scalatestplus" %% "mockito-4-11" % "3.2.17.0" % Test
)

// Assembly plugin for fat JAR
assembly / assemblyMergeStrategy := {
  case PathList("META-INF", "versions", xs @ _*) => MergeStrategy.first
  case PathList("META-INF", "services", xs @ _*) => MergeStrategy.filterDistinctLines
  case PathList("META-INF", xs @ _*) => MergeStrategy.discard
  case "reference.conf" => MergeStrategy.concat
  case _ => MergeStrategy.first
}

// Exclude provided dependencies from assembly
assembly / assemblyExcludedJars := {
  val cp = (assembly / fullClasspath).value
  cp filter { jar =>
    jar.data.getName.contains("flink-") && 
    !jar.data.getName.contains("flink-connector-base")
  }
}

// Assembly settings
assembly / assemblyJarName := "flink-cosmos-connector-scala-1.0-SNAPSHOT.jar"
assembly / test := {}

// Compiler options
scalacOptions ++= Seq(
  "-feature",
  "-deprecation",
  "-unchecked",
  "-language:higherKinds",
  "-language:implicitConversions"
)

// Java compatibility
javacOptions ++= Seq("-source", "17", "-target", "17")