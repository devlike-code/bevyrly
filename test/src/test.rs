fn test_system(s: Query<&A, With<P>>) {
    println!("{:?}", s);
}

fn test_query(q: Query<(Entity, &Transform, &mut Velocity)>) {
    println!("{:?}", q);
}

fn test_generic<CT: Component>(q: Query<&CT>) {
    println!("{:?}", q);
}

fn res_test(mut comm: Commands, r: Option<Res<B>>, p: ResMut<P>) {}

struct Foo;

impl Foo {
    fn method_system(q: Query<Entity>) {}
}
