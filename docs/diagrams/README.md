# Oxygen Protocol: Diagrams

This directory contains UML diagrams and other visual representations of the Oxygen Protocol architecture and workflows.

## Available Diagrams

### System Architecture

- [System Architecture Diagram](./system_architecture.png) - High-level overview of the protocol components
- [System Architecture (PlantUML)](./system_architecture.puml) - PlantUML source file

### Data Flow Diagrams

- [Deposit Flow](./deposit_flow.png) - Sequence diagram of the deposit process
- [Borrow Flow](./borrow_flow.png) - Sequence diagram of the borrow process
- [Withdraw Flow](./withdraw_flow.png) - Sequence diagram of the withdraw process
- [Liquidation Flow](./liquidation_flow.png) - Sequence diagram of the liquidation process

### Class Diagrams

- [Protocol Core Classes](./core_classes.png) - Class diagram of core protocol components
- [SDK Classes](./sdk_classes.png) - Class diagram of SDK components

### State Diagrams

- [Position State Diagram](./position_states.png) - States a user position can transition through
- [Pool State Diagram](./pool_states.png) - States a lending pool can transition through

### Miscellaneous

- [Interest Rate Model](./interest_rate_model.png) - Visualization of the interest rate model
- [Health Factor Model](./health_factor.png) - Visualization of the health factor calculation

## Using These Diagrams

### For Developers

These diagrams are intended to help developers understand the protocol architecture for integration or contribution purposes. The PlantUML source files are provided for each diagram, allowing you to modify and regenerate them as needed.

### For Users

If you're a user looking to understand how the protocol works, we recommend starting with the data flow diagrams, which provide a clear visualization of how your interactions with the protocol are processed.

## Updating Diagrams

The diagrams are generated using PlantUML. To update a diagram:

1. Modify the `.puml` source file
2. Generate the diagram using PlantUML:

```bash
plantuml diagram_name.puml
```

3. Commit both the updated `.puml` file and the resulting image

## Additional Resources

- [PlantUML Documentation](https://plantuml.com/en/)
- [Architecture Overview](../architecture/overview.md) - Detailed explanation of the protocol architecture